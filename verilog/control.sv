// control.sv
module control
    import accel_pkg::*;
(
    // 基本インターフェース
    input  logic clk,
    input  logic rst_n,

    // システム制御
    input  logic [7:0] sys_control,
    output logic [7:0] sys_status,

    // 最適化されたユニット制御インターフェース
    output control_packet_t [NUM_PROCESSING_UNITS-1:0] unit_control,
    input  logic [NUM_PROCESSING_UNITS-1:0] unit_ready,
    input  logic [NUM_PROCESSING_UNITS-1:0] unit_done,

    // パフォーマンスモニタリング
    output logic [15:0] perf_counter
);
    // システム状態の定義
    typedef enum logic [2:0] {
        ST_IDLE,
        ST_INIT,
        ST_DISPATCH,
        ST_EXECUTE,
        ST_SYNC
    } sys_state_t;

    // 内部状態と制御信号
    sys_state_t current_state;
    logic [3:0] active_units;
    logic [15:0] cycle_counter;
    
    // メインステートマシン
    always_ff @(posedge clk or negedge rst_n) begin
        if (!rst_n) begin
            reset_system();
        end
        else begin
            // サイクルカウンタの更新
            cycle_counter <= cycle_counter + 1;
            
            // ステート遷移
            case (current_state)
                ST_IDLE:     handle_idle_state();
                ST_INIT:     handle_init_state();
                ST_DISPATCH: handle_dispatch_state();
                ST_EXECUTE:  handle_execute_state();
                ST_SYNC:     handle_sync_state();
            endcase

            // ステータス更新
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
        
        // 全ユニットの制御信号をクリア
        for (int i = 0; i < NUM_PROCESSING_UNITS; i++) begin
            unit_control[i] <= '0;
        end
    endtask

    // IDLE状態のハンドリング
    task handle_idle_state();
        if (sys_control[0]) begin  // システム起動信号
            current_state <= ST_INIT;
            active_units <= '0;
        end
    endtask

    // 初期化状態のハンドリング
    task handle_init_state();
        // 全ユニットの初期化
        for (int i = 0; i < NUM_PROCESSING_UNITS; i++) begin
            if (unit_ready[i]) begin
                // NOP命令の設定
                unit_control[i].encoded_control <= {i[1:0], 2'b00, 2'b00};
                unit_control[i].data_control <= '0;
                active_units[i] <= 1'b0;
            end
        end
        current_state <= ST_DISPATCH;
    endtask

    // タスク割り当て状態のハンドリング
    task handle_dispatch_state();
        for (int i = 0; i < NUM_PROCESSING_UNITS; i++) begin
            if (unit_ready[i] && !active_units[i]) begin
                // データ転送モードか計算モードかを判定
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
        unit_control[unit_index].encoded_control <= {
            unit_index[1:0],         // ユニットID
            2'b11,                    // 計算操作
            sys_control[3:2]          // 計算タイプ
        };
        unit_control[unit_index].data_control <= {
            sys_control[7:4],         // データアドレス
            1'b1,                     // 有効フラグ
            3'b111                    // 最大サイズ
        };
        active_units[unit_index] <= 1'b1;
    endtask

    // データ転送制御の設定
    task set_transfer_control(input int unit_index);
        unit_control[unit_index].encoded_control <= {
            unit_index[1:0],          // ユニットID
            sys_control[5] ? 2'b10 : 2'b01,  // ストアかロード
            2'b00                     // 未使用
        };
        unit_control[unit_index].data_control <= {
            sys_control[7:4],         // データアドレス
            1'b1,                     // 有効フラグ
            3'b111                    // 最大サイズ
        };
    endtask

    // 実行状態のハンドリング
    task handle_execute_state();
        // アクティブユニットの完了を確認
        for (int i = 0; i < NUM_PROCESSING_UNITS; i++) begin
            if (unit_done[i]) begin
                active_units[i] <= 1'b0;
                // 完了したユニットにNOP命令を設定
                unit_control[i].encoded_control <= {i[1:0], 2'b00, 2'b00};
            end
        end

        // 全ユニットが完了したら同期状態へ
        if ((active_units & ~unit_ready) == '0) begin
            current_state <= ST_SYNC;
        end
    endtask

    // 同期状態のハンドリング
    task handle_sync_state();
        // パフォーマンスカウンタの更新
        perf_counter <= cycle_counter;
        
        // 継続フラグに応じて状態遷移
        if (sys_control[7]) begin  // 継続フラグ
            current_state <= ST_DISPATCH;
        end
        else begin
            current_state <= ST_IDLE;
        end
    endtask

    // システムステータスの更新
    task update_system_status();
        sys_status <= {
            current_state != ST_IDLE,    // ビジー
            |active_units,               // アクティブユニット存在
            current_state == ST_SYNC,    // 同期状態
            5'b0                         // 予約
        };
    endtask

    // デバッグ用パフォーマンスモニタリング
    // synthesis translate_off
    always_ff @(posedge clk) begin
        if (sys_status[7]) begin  // ビジー状態
            $display("システムステータス: アクティブユニット %b", active_units);
            $display("パフォーマンスカウンタ: %0d サイクル", perf_counter);
        end
    end
    // synthesis translate_on
endmodule