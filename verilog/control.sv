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
    // 内部状態
    typedef enum logic [2:0] {
        SYS_IDLE,
        SYS_INIT,
        SYS_DISPATCH,
        SYS_WAIT,
        SYS_SYNC
    } sys_state_t;

    sys_state_t current_state;
    logic [3:0] active_units;
    logic [15:0] cycle_counter;
    
    // デコーダインスタンス
    decoder_unit u_decoder [NUM_PROCESSING_UNITS-1:0] (
        .clk(clk),
        .rst_n(rst_n),
        .instruction_packet({unit_control[0].encoded_control, unit_control[0].data_control}),
        .decode_valid(),
        .error_status()
    );

    // システム状態管理
    always_ff @(posedge clk or negedge rst_n) begin
        if (!rst_n) begin
            current_state <= SYS_IDLE;
            active_units <= '0;
            cycle_counter <= '0;
            sys_status <= '0;
            perf_counter <= '0;
            for (int i = 0; i < NUM_PROCESSING_UNITS; i++) begin
                unit_control[i] <= '0;
            end
        end
        else begin
            // サイクルカウンタ更新
            cycle_counter <= cycle_counter + 1;
            
            case (current_state)
                SYS_IDLE: begin
                    if (sys_control[0]) begin
                        current_state <= SYS_INIT;
                        active_units <= '0;
                    end
                end

                SYS_INIT: begin
                    // ユニットの初期化（エンコードされた制御信号）
                    for (int i = 0; i < NUM_PROCESSING_UNITS; i++) begin
                        if (unit_ready[i]) begin
                            unit_control[i].encoded_control <= {i[1:0], 2'b00, 2'b00}; // NOP命令
                            unit_control[i].data_control <= '0;
                            active_units[i] <= 1'b0;
                        end
                    end
                    current_state <= SYS_DISPATCH;
                end

                SYS_DISPATCH: begin
                    // タスク割り当て（最適化された制御パケット）
                    for (int i = 0; i < NUM_PROCESSING_UNITS; i++) begin
                        if (unit_ready[i] && !active_units[i]) begin
                            if (sys_control[1]) begin  // 計算モード
                                unit_control[i].encoded_control <= {
                                    i[1:0],                    // unit_id
                                    2'b11,                     // OP_COMP
                                    sys_control[3:2]           // comp_type
                                };
                                unit_control[i].data_control <= {
                                    sys_control[7:4],          // data_addr
                                    1'b1,                      // valid
                                    3'b111                     // full size
                                };
                                active_units[i] <= 1'b1;
                            end
                            else begin  // データ転送モード
                                unit_control[i].encoded_control <= {
                                    i[1:0],                    // unit_id
                                    sys_control[5] ? 2'b10 : 2'b01,  // OP_STORE or OP_LOAD
                                    2'b00                      // unused
                                };
                                unit_control[i].data_control <= {
                                    sys_control[7:4],          // data_addr
                                    1'b1,                      // valid
                                    3'b111                     // full size
                                };
                            end
                        end
                    end
                    current_state <= SYS_WAIT;
                end

                SYS_WAIT: begin
                    // 完了待ち
                    for (int i = 0; i < NUM_PROCESSING_UNITS; i++) begin
                        if (unit_done[i]) begin
                            active_units[i] <= 1'b0;
                            unit_control[i].encoded_control <= {i[1:0], 2'b00, 2'b00}; // NOP命令
                        end
                    end

                    if ((active_units & ~unit_ready) == '0) begin
                        current_state <= SYS_SYNC;
                    end
                end

                SYS_SYNC: begin
                    // パフォーマンスカウンタの更新
                    perf_counter <= cycle_counter;
                    
                    if (sys_control[7]) begin  // 継続フラグ
                        current_state <= SYS_DISPATCH;
                    end
                    else begin
                        current_state <= SYS_IDLE;
                    end
                end

                default: current_state <= SYS_IDLE;
            endcase

            // ステータス更新
            sys_status <= {
                current_state != SYS_IDLE,    // ビジー
                |active_units,                // アクティブユニット存在
                current_state == SYS_SYNC,    // 同期状態
                5'b0                          // 予約
            };
        end
    end

endmodule