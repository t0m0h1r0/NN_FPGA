// common.sv
module status_control 
    import accel_pkg::*;
(
    input  logic clk,
    input  logic rst_n,
    input  logic start,
    input  logic [4:0] max_count,
    output logic busy,
    output logic done,
    output logic [4:0] counter
);
    // 共通ステータス制御
    always_ff @(posedge clk or negedge rst_n) begin
        if (!rst_n) begin
            busy <= 1'b0;
            done <= 1'b0;
            counter <= '0;
        end
        else begin
            if (start && !busy) begin
                busy <= 1'b1;
                done <= 1'b0;
                counter <= '0;
            end
            else if (busy) begin
                if (counter == max_count) begin
                    busy <= 1'b0;
                    done <= 1'b1;
                    counter <= '0;
                end
                else begin
                    counter <= counter + 1;
                end
            end
            else begin
                done <= 1'b0;
            end
        end
    end
endmodule

// 共通ベクトル演算モジュール
module vector_alu 
    import accel_pkg::*;
(
    input  logic clk,
    input  computation_type_t op_type,
    input  logic [VECTOR_WIDTH-1:0] a,
    input  logic [VECTOR_WIDTH-1:0] b,
    output logic [VECTOR_WIDTH-1:0] result
);
    logic signed [2*VECTOR_WIDTH-1:0] mult_result;
    
    always_comb begin
        case (op_type)
            COMP_ADD: result = a + b;
            COMP_MUL: result = mult_result[VECTOR_WIDTH-1:0];
            COMP_RELU: result = a[VECTOR_WIDTH-1] ? '0 : a;
            COMP_TANH: result = {a[VECTOR_WIDTH-1], {(VECTOR_WIDTH-1){~a[VECTOR_WIDTH-1]}}};
            default: result = a;
        endcase
    end

    // 乗算ロジック
    always_ff @(posedge clk) begin
        mult_result <= $signed(a) * $signed(b);
    end
endmodule