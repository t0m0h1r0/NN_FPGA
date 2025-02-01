// top.sv
module top
    import accel_pkg::*;
(
    // システムインターフェース
    input  logic clk,
    input  logic rst_n,
    input  logic [7:0] sys_control,
    output logic [7:0] sys_status,
    output logic [15:0] perf_counter,

    // データインターフェース
    input  vector_data_t data_in [NUM_PROCESSING_UNITS],
    input  matrix_data_t matrix_in [NUM_PROCESSING_UNITS],
    output vector_data_t data_out [NUM_PROCESSING_UNITS]
);
    // 内部接続信号
    control_packet_t [NUM_PROCESSING_UNITS-1:0] unit_control;
    logic [NUM_PROCESSING_UNITS-1:0] unit_ready;
    logic [NUM_PROCESSING_UNITS-1:0] unit_done;

    // システムコントローラのインスタンス化
    control u_control (
        .clk(clk),
        .rst_n(rst_n),
        .sys_control(sys_control),
        .sys_status(sys_status),
        .unit_control(unit_control),
        .unit_ready(unit_ready),
        .unit_done(unit_done),
        .perf_counter(perf_counter)
    );

    // 処理ユニットの生成
    generate
        for (genvar i = 0; i < NUM_PROCESSING_UNITS; i++) begin : gen_units
            unit u_unit (
                .clk(clk),
                .rst_n(rst_n),
                .control(unit_control[i]),
                .ready(unit_ready[i]),
                .done(unit_done[i]),
                .data_in(data_in[i]),
                .matrix_in(matrix_in[i]),
                .data_out(data_out[i])
            );
        end
    endgenerate

    // デバッグ用パフォーマンスモニタリング
    // synthesis translate_off
    always_ff @(posedge clk) begin
        if (sys_status[7]) begin  // ビジー状態
            $display("Time=%0t: Active processing units: %b", $time, unit_ready);
            $display("Performance counter: %0d cycles", perf_counter);
        end
    end
    // synthesis translate_on

endmodule