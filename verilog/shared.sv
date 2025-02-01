// shared.sv
module shared
    import accel_pkg::*;
(
    input  logic clk,
    input  logic rst_n,
    input  computation_type_t comp_type,
    input  logic start,
    output logic busy,
    output logic done,
    input  vector_data_t vector_a,
    input  vector_data_t vector_b,
    input  matrix_data_t matrix_in,
    output vector_data_t result
);
    // 内部信号
    logic [4:0] compute_counter;
    vector_data_t temp_result;
    
    // ステータス制御インスタンス
    status_control u_status (
        .clk(clk),
        .rst_n(rst_n),
        .start(start),
        .max_count(5'd16),
        .busy(busy),
        .done(done),
        .counter(compute_counter)
    );

    // ベクトルALUインスタンス
    vector_alu u_alu (
        .clk(clk),
        .op_type(comp_type),
        .a(vector_a.data[compute_counter]),
        .b(vector_b.data[compute_counter]),
        .result(temp_result.data[compute_counter])
    );

    // 行列演算用一時データ
    logic [VECTOR_WIDTH-1:0] matrix_product;
    logic matrix_valid;

    // 行列乗算の制御
    always_ff @(posedge clk) begin
        if (comp_type == COMP_MUL) begin
            if (busy) begin
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
        end
        else begin
            matrix_valid <= 1'b0;
        end
    end

    // 結果の選択
    assign result = (done) ? temp_result : '0;

endmodule