// top.sv
module accelerator_top
    import accel_pkg::*;
(
    // システム基本インターフェース
    input  logic clk,
    input  logic rst_n,
    input  logic [7:0] sys_control,
    output logic [7:0] sys_status,
    output logic [15:0] perf_counter,

    // データインターフェース
    input  vector_t data_in [UNIT_COUNT],
    input  matrix_t matrix_in [UNIT_COUNT],
    output vector_t data_out [UNIT_COUNT]
);
    // 内部接続信号
    ctrl_packet_t [UNIT_COUNT-1:0] unit_control;
    logic [UNIT_COUNT-1:0] unit_ready;
    logic [UNIT_COUNT-1:0] unit_done;
    
    // 共有リソース制御信号
    logic [UNIT_COUNT-1:0] unit_compute_request;
    logic compute_ready;
    logic compute_done;
    comp_type_e current_comp_type;
    vector_t current_vector_a;
    vector_t current_vector_b;
    matrix_t current_matrix;
    vector_t compute_result;

    // システムコントローラ
    system_controller u_system_controller (
        .clk(clk),
        .rst_n(rst_n),
        .sys_control(sys_control),
        .sys_status(sys_status),
        .unit_control(unit_control),
        .unit_ready(unit_ready),
        .unit_done(unit_done),
        .perf_counter(perf_counter)
    );

    // 共有計算ユニット
    shared_compute_unit u_shared_compute (
        .clk(clk),
        .rst_n(rst_n),
        .unit_id('0),  // 8ビットのユニットID
        .request(unit_compute_request[0]),
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
        for (genvar i = 0; i < UNIT_COUNT; i++) begin : gen_processing_units
            processing_unit u_unit (
                .clk(clk),
                .rst_n(rst_n),
                .unit_id(i[UNIT_ID_WIDTH-1:0]),
                .control(unit_control[i]),
                .ready(unit_ready[i]),
                .done(unit_done[i]),
                .data_in(data_in[i]),
                .matrix_in(matrix_in[i]),
                .data_out(data_out[i])
            );
        end
    endgenerate

    // リソース間の接続ロジック
    always_comb begin
        // 共有計算ユニットへのデータ接続
        unit_compute_request = '0;
        current_vector_a = '0;
        current_vector_b = '0;
        current_matrix = '0;
        current_comp_type = COMP_ADD;

        // システムコントローラからの制御に基づいて接続
        if (sys_control[1]) begin  // 計算モード
            unit_compute_request[0] = 1'b1;
            current_vector_a = data_in[0];
            current_vector_b = data_in[1];
            current_matrix = matrix_in[0];
            current_comp_type = comp_type_e'(sys_control[3:2]);
        end
    end

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