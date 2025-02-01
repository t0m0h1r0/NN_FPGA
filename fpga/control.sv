// control.sv
module system_controller
    import accel_pkg::*;
(
    // 基本システムインターフェース
    input  logic clk,
    input  logic rst_n,

    // システム制御
    input  logic [7:0] sys_control,
    output logic [7:0] sys_status,

    // ユニット制御インターフェース
    output ctrl_packet_t [UNIT_COUNT-1:0] unit_control,
    input  logic [UNIT_COUNT-1:0] unit_ready,
    input  logic [UNIT_COUNT-1:0] unit_done,

    // パフォーマンスモニタリング
    output logic [15:0] perf_counter
);
    // システム状態定義
    typedef enum logic [2:0] {
        ST_IDLE,
        ST_INIT,
        ST_DISPATCH,
        ST_EXECUTE,
        ST_SYNC
    } sys_state_e;

    // システム内部状態
    sys_state_e current_state;
    logic [3:0] active_units;
    logic [15:0] cycle_counter;
    logic [UNIT_COUNT-1:0] unit_priority;

    // メインステートマシン
    always_ff @(posedge clk or negedge rst_n) begin
        if (!rst_n) begin
            reset_system();
        end
        else begin
            // サイクルカウンタ更新
            cycle_counter <= cycle_counter + 1;
            
            // ステート遷移
            case (current_state)
                ST_IDLE:     handle_idle_state();
                ST_INIT:     handle_init_state();
                ST_DISPATCH: handle_dispatch_state();
                ST_EXECUTE:  handle_execute_state();
                ST_SYNC:     handle_sync_state();
            endcase

            // システムステータス更新
            update_system_status();
        end
    end

    // システムリセットタスク
    task reset_system();
        current_state <= ST_IDLE;
        active_units <= '0;
        cycle_counter <= '0;
        sys_status <= '0;
        perf_counter <= '0;
        unit_priority <= {UNIT_COUNT{1'b1}};  // 全ユニットに初期優先度
        
        // 全ユニット制御信号のクリア
        for (int i = 0; i < UNIT_COUNT; i++) begin
            unit_control[i] <= '0;
        end
    endtask

    // アイドル状態ハンドリング
    task handle_idle_state();
        if (sys_control[0]) begin  // システム起動信号
            current_state <= ST_INIT;
            active_units <= '0;
        end
    endtask

    // 初期化状態ハンドリング
    task handle_init_state();
        // 全ユニットの初期化
        for (int i = 0; i < UNIT_COUNT; i++) begin
            if (unit_ready[i]) begin
                // NOP命令の設定
                unit_control[i].ctrl <= {i[1:0], 2'b00, 2'b00};
                unit_control[i].config <= '0;
            end
        end
        current_state <= ST_DISPATCH;
    endtask

    // タスク割り当て状態ハンドリング
    task handle_dispatch_state();
        // 優先度に基づくタスク割り当て
        for (int i = 0; i < UNIT_COUNT; i++) begin
            if (unit_ready[i] && !active_units[i] && unit_priority[i]) begin
                // 計算モードかデータ転送モードかを判定
                if (sys_control[1]) begin  // 計算モード
                    set_compute_control(i);
                end
                else begin  // データ転送モード
                    set_transfer_control(i);
                end
            end
        end
        current_state <= ST_EXECUTE;
    endtask

    // 計算制御の設定
    task set_compute_control(input int unit_index);
        unit_control[unit_index].ctrl <= {
            unit_index[1:0],         // ユニットID
            2'b11,                    // 計算操作
            sys_control[3:2]          // 計算タイプ
        };
        unit_control[unit_index].config <= {
            sys_control[7:4],         // データアドレス
            1'b1,                     // 有効フラグ
            3'b111                    // 最大サイズ
        };
        active_units[unit_index] <= 1'b1;
        // 使用後に優先度を下げる
        unit_priority[unit_index] <= 1'b0;
    endtask

    // データ転送制御の設定
    task set_transfer_control(input int unit_index);
        unit_control[unit_index].ctrl <= {
            unit_index[1:0],          // ユニットID
            sys_control[5] ? 2'b10 : 2'b01,  // ストアかロード
            2'b00                     // 未使用
        };
        unit_control[unit_index].config <= {
            sys_control[7:4],         // データアドレス
            1'b1,                     // 有効フラグ
            3'b111                    // 最大サイズ
        };
        active_units[unit_index] <= 1'b1;
        // 使用後に優先度を下げる
        unit_priority[unit_index] <= 1'b0;
    endtask

    // 実行状態ハンドリング
    task handle_execute_state();
        // ユニットの完了と優先度の管理
        for (int i = 0; i < UNIT_COUNT; i++) begin
            if (unit_done[i]) begin
                active_units[i] <= 1'b0;
                // 完了したユニットにNOP命令を設定し、優先度を復元
                unit_control[i].ctrl <= {i[1:0], 2'b00, 2'b00};
                unit_priority[i] <= 1'b1;
            end
        end

        // 全ユニットが完了したら同期状態へ
        if ((active_units & ~unit_ready) == '0) begin
            current_state <= ST_SYNC;
        end
    endtask

    // 同期状態ハンドリング
    task handle_sync_state();
        // パフォーマンスカウンタの更新
        perf_counter <= cycle_counter;
        
        // 継続フラグに応じて状態遷移
        if (sys_control[7]) begin  // 継続フラグ
            current_state <= ST_DISPATCH;
            // 全ユニットの優先度を復元
            unit_priority <= {UNIT_COUNT{1'b1}};
        end
        else begin
            current_state <= ST_IDLE;
        end
    endtask

    // システムステータス更新
    task update_system_status();
        sys_status <= {
            current_state != ST_IDLE,    // システムビジー
            |active_units,               // アクティブユニット存在
            current_state == ST_SYNC,    // 同期状態
            5'b0                         // 予約
        };
    endtask

endmodule