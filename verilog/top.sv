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
    
    // 共有演算ユニット用信号
    logic [1:0] current_unit_id;
    logic [NUM_PROCESSING_UNITS-1:0] unit_compute_request;
    logic compute_ready;
    logic compute_done;
    computation_type_t current_comp_type;
    vector_data_t current_vector_a;
    vector_data_t current_vector_b;
    matrix_data_t current_matrix;
    vector_data_t compute_result;

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

    // 共有演算ユニット
    shared_compute_unit u_compute (
        .clk(clk),
        .rst_n(rst_n),
        .unit_id(current_unit_id),
        .request(unit_compute_request[current_unit_id]),
        .ready(compute_ready),
        .done(compute_done),
        .comp_type(current_comp_type),
        .vector_a(current_vector_a),
        .vector_b(current_vector_b),
        .matrix_in(current_matrix),
        .result(compute_result)
    );

    // 処理ユニットの生成
    generate
        for (genvar i = 0; i < NUM_PROCESSING_UNITS; i++) begin : gen_units
            unit u_unit (
                .clk(clk),
                .rst_n(rst_n),
                .unit_id(i[1:0]),
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
            $display("アクティブ処理ユニット: %b", unit_ready);
            $display("パフォーマンスカウンタ: %0d サイクル", perf_counter);
        end
    end
    // synthesis translate_on
endmodule