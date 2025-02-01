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
    input  data_t data_in [UNIT_COUNT],
    output data_t data_out [UNIT_COUNT]
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
    data_t current_data;
    data_t compute_result;

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
        .unit_id('0),
        .request(unit_compute_request[0]),
        .ready(compute_ready),
        .done(compute_done),
        .comp_type(current_comp_type),
        .data_in(current_data),
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
                .data_out(data_out[i])
            );
        end
    endgenerate

    // リソース間の接続ロジック
    always_comb begin
        // 共有計算ユニットへのデータ接続
        unit_compute_request = '0;
        current_data = '0;
        current_comp_type = COMP_ADD;

        // システムコントローラからの制御に基づいて接続
        if (sys_control[1]) begin  // 計算モード
            unit_compute_request[0] = 1'b1;
            current_data = data_in[0];
            current_comp_type = comp_type_e'(sys_control[3:2]);
        end
    end

endmodule