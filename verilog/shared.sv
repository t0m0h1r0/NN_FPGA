// shared.sv
module shared_compute_unit
    import accel_pkg::*;
(
    input  logic clk,
    input  logic rst_n,
    
    // 制御インターフェース
    input  logic [1:0] unit_id,         // 要求元ユニットのID
    input  logic request,               // 演算要求
    output logic ready,                 // 演算器使用可能
    output logic done,                  // 演算完了
    
    // データインターフェース
    input  computation_type_t comp_type,
    input  vector_data_t vector_a,
    input  vector_data_t vector_b,
    input  matrix_data_t matrix_in,
    output vector_data_t result
);
    // 優先順位制御
    logic [3:0] unit_priority;
    logic [1:0] current_unit;
    logic processing;

    // 内部信号
    logic [4:0] compute_counter;
    vector_data_t temp_result;
    
    // アービトレーション
    always_ff @(posedge clk or negedge rst_n) begin
        if (!rst_n) begin
            unit_priority <= 4'b0001; // 初期優先順位
            current_unit <= 2'b00;
            processing <= 1'b0;
            ready <= 1'b1;
        end
        else begin
            if (!processing && request) begin
                // 新しい要求を受け付け
                current_unit <= unit_id;
                processing <= 1'b1;
                ready <= 1'b0;
                // 優先順位の更新
                unit_priority <= {unit_priority[2:0], unit_priority[3]};
            end
            else if (done) begin
                processing <= 1'b0;
                ready <= 1'b1;
            end
        end
    end

    // 演算制御
    status_control u_status (
        .clk(clk),
        .rst_n(rst_n),
        .start(processing),
        .max_count(5'd16),
        .busy(processing),
        .done(done),
        .counter(compute_counter)
    );

    // ベクトルALU
    vector_alu u_alu (
        .clk(clk),
        .op_type(comp_type),
        .a(vector_a.data[compute_counter]),
        .b(vector_b.data[compute_counter]),
        .result(temp_result.data[compute_counter])
    );

    // 行列演算用
    logic [VECTOR_WIDTH-1:0] matrix_product;
    logic matrix_valid;

    always_ff @(posedge clk) begin
        if (comp_type == COMP_MUL && processing) begin
            matrix_product <= '0;
            matrix_valid <= 1'b1;
            for (int j = 0; j < MATRIX_DEPTH; j++) begin
                if (matrix_in.data[compute_counter][j][0]) begin
                    matrix_product <= matrix_product + 
                        (matrix_in.data[compute_counter][j][1] ? 
                         -vector_a.data[j] : vector_a.data[j]);
                end
            end
        end
        else begin
            matrix_valid <= 1'b0;
        end
    end

    // 結果の選択
    assign result = (done && current_unit == unit_id) ? temp_result : '0;

endmodule