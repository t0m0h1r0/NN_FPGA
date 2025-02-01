// unit.sv
module unit
    import accel_pkg::*;
(
    // 基本インターフェース
    input  logic clk,
    input  logic rst_n,
    input  logic [1:0] unit_id,
    
    // 最適化された制御インターフェース
    input  control_packet_t control,
    output logic ready,
    output logic done,
    
    // メモリインターフェース
    output logic mem_request,
    output logic [3:0] mem_op_type,
    output logic [3:0] vec_index,
    output logic [3:0] mat_row,
    output logic [3:0] mat_col,
    output vector_data_t write_data,
    input  logic mem_grant,
    input  vector_data_t read_data,
    input  logic mem_done,
    
    // データインターフェース
    input  vector_data_t data_in,
    input  matrix_data_t matrix_in,
    output vector_data_t data_out
);
    // ローカル制御信号
    control_signal_t decoded_control;
    logic decode_valid;
    logic [1:0] error_status;

    // デコーダインスタンス
    decoder_unit u_decoder (
        .clk(clk),
        .rst_n(rst_n),
        .instruction_packet({control.encoded_control, control.data_control}),
        .decoded_control(decoded_control),
        .decode_valid(decode_valid),
        .error_status(error_status)
    );

    // 内部状態とレジスタ
    unit_state_t current_state;
    logic compute_pending;
    vector_data_t temp_vector;
    matrix_data_t temp_matrix;

    // 状態マシン
    always_ff @(posedge clk or negedge rst_n) begin
        if (!rst_n) begin
            current_state <= IDLE;
            compute_pending <= 1'b0;
            ready <= 1'b1;
            done <= 1'b0;
            mem_request <= 1'b0;
            mem_op_type <= '0;
            vec_index <= '0;
            mat_row <= '0;
            mat_col <= '0;
            write_data <= '0;
            data_out <= '0;
        end
        else begin
            case (current_state)
                IDLE: begin
                    if (decode_valid && decoded_control.valid) begin
                        ready <= 1'b0;
                        current_state <= TRANSFER;
                        
                        // メモリアクセス設定
                        case (decoded_control.op_code)
                            OP_LOAD: begin
                                mem_request <= 1'b1;
                                mem_op_type <= 4'b0001;
                                vec_index <= decoded_control.addr;
                            end
                            OP_STORE: begin
                                mem_request <= 1'b1;
                                mem_op_type <= 4'b0010;
                                vec_index <= decoded_control.addr;
                                write_data <= data_in;
                            end
                            OP_COMP: begin
                                compute_pending <= 1'b1;
                                temp_matrix <= matrix_in;
                                mem_request <= 1'b1;
                                mem_op_type <= 4'b0100;
                            end
                            default: begin
                                current_state <= IDLE;
                                ready <= 1'b1;
                            end
                        endcase
                    end
                end

                TRANSFER: begin
                    mem_request <= 1'b0;
                    if (mem_done) begin
                        if (compute_pending) begin
                            current_state <= COMPUTE;
                            temp_vector <= read_data;
                        end
                        else begin
                            if (decoded_control.op_code == OP_LOAD) begin
                                data_out <= read_data;
                            end
                            done <= 1'b1;
                            current_state <= IDLE;
                            ready <= 1'b1;
                        end
                    end
                end

                COMPUTE: begin
                    case (decoded_control.comp_type)
                        COMP_ADD: begin
                            for (int i = 0; i < VECTOR_DEPTH; i++) begin
                                data_out.data[i] <= temp_vector.data[i] + read_data.data[i];
                            end
                        end
                        COMP_MUL: begin
                            logic [VECTOR_WIDTH-1:0] sum;
                            for (int i = 0; i < VECTOR_DEPTH; i++) begin
                                sum = '0;
                                for (int j = 0; j < MATRIX_DEPTH; j++) begin
                                    if (temp_matrix.data[i][j][0]) begin
                                        sum = sum + (temp_matrix.data[i][j][1] ? 
                                            -temp_vector.data[j] : temp_vector.data[j]);
                                    end
                                end
                                data_out.data[i] <= sum;
                            end
                        end
                        COMP_TANH: begin
                            for (int i = 0; i < VECTOR_DEPTH; i++) begin
                                data_out.data[i] <= temp_vector.data[i][VECTOR_WIDTH-1] ? 
                                    {1'b1, {(VECTOR_WIDTH-1){1'b0}}} :
                                    {1'b0, {(VECTOR_WIDTH-1){1'b1}}};
                            end
                        end
                        COMP_RELU: begin
                            for (int i = 0; i < VECTOR_DEPTH; i++) begin
                                data_out.data[i] <= temp_vector.data[i][VECTOR_WIDTH-1] ? 
                                    '0 : temp_vector.data[i];
                            end
                        end
                    endcase
                    compute_pending <= 1'b0;
                    done <= 1'b1;
                    current_state <= IDLE;
                    ready <= 1'b1;
                end
            endcase

            // エラー処理
            if (|error_status) begin
                current_state <= IDLE;
                ready <= 1'b1;
                done <= 1'b0;
                mem_request <= 1'b0;
                compute_pending <= 1'b0;
            end
        end
    end

    // synthesis translate_off
    // デバッグ用モニタリング
    always @(posedge clk) begin
        if (|error_status) begin
            $display("Unit %0d Error: state=%0d, error=0x%h", 
                    unit_id, current_state, error_status);
        end
    end
    // synthesis translate_on

endmodule