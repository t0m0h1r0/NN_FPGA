// unit.sv
module unit
    import accel_pkg::*;
(
    // 基本インターフェース
    input  logic clk,
    input  logic rst_n,
    input  logic [1:0] unit_id,        // ユニットID追加
    
    // 制御インターフェース
    input  control_packet_t control,
    output logic ready,
    output logic done,
    
    // 演算器共有インターフェース
    output logic compute_request,       // 演算要求
    input  logic compute_ready,         // 演算器使用可能
    input  logic compute_done,         // 演算完了
    
    // データインターフェース
    input  vector_data_t data_in,
    input  matrix_data_t matrix_in,
    output vector_data_t data_out
);

    // 内部状態とメモリ
    unit_state_t current_state;
    vector_data_t local_vector;
    matrix_data_t local_matrix;

    always_ff @(posedge clk or negedge rst_n) begin
        if (!rst_n) begin
            current_state <= IDLE;
            compute_request <= 1'b0;
            ready <= 1'b1;
            done <= 1'b0;
            local_vector <= '0;
            local_matrix <= '0;
        end
        else begin
            case (current_state)
                IDLE: begin
                    if (control.op_code != OP_NOP && ready) begin
                        ready <= 1'b0;
                        current_state <= FETCH;
                    end
                    compute_request <= 1'b0;
                    done <= 1'b0;
                end

                FETCH: begin
                    case (control.op_code)
                        OP_LOAD: begin
                            local_vector <= data_in;
                            current_state <= WRITEBACK;
                        end
                        OP_STORE: begin
                            data_out <= local_vector;
                            current_state <= WRITEBACK;
                        end
                        OP_COMP: begin
                            if (compute_ready) begin
                                local_matrix <= matrix_in;
                                compute_request <= 1'b1;
                                current_state <= COMPUTE;
                            end
                        end
                        default: current_state <= IDLE;
                    endcase
                end

                COMPUTE: begin
                    compute_request <= 1'b0;
                    if (compute_done) begin
                        current_state <= WRITEBACK;
                    end
                end

                WRITEBACK: begin
                    done <= 1'b1;
                    ready <= 1'b1;
                    current_state <= IDLE;
                end

                default: current_state <= IDLE;
            endcase
        end
    end

endmodule