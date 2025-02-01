// unit.sv
module processing_unit
    import accel_pkg::*;
(
    input  logic clk,
    input  logic rst_n,
    input  logic [1:0] unit_id,
    
    // 制御インターフェース
    input  ctrl_packet_t control,
    output logic ready,
    output logic done,
    
    // メモリインターフェース
    output logic mem_request,
    output logic [3:0] mem_op_type,
    output logic [3:0] vec_index,
    output logic [3:0] mat_row,
    output logic [3:0] mat_col,
    output vector_t write_data,
    input  logic mem_grant,
    input  vector_t read_data,
    input  logic mem_done,
    
    // データインターフェース
    input  vector_t data_in,
    input  matrix_t matrix_in,
    output vector_t data_out
);
    // 内部状態
    typedef enum logic [1:0] {
        ST_IDLE,
        ST_FETCH,
        ST_EXECUTE,
        ST_WRITEBACK
    } unit_state_e;

    // デコーダ
    decoded_ctrl_t decoded_ctrl;
    logic decode_valid;
    logic [1:0] error_status;

    decoder u_decoder (
        .clk(clk),
        .rst_n(rst_n),
        .ctrl_packet(control),
        .decoded_ctrl(decoded_ctrl),
        .decode_valid(decode_valid),
        .error_status(error_status)
    );

    // 内部状態レジスタ
    unit_state_e current_state;
    vector_t temp_vector;
    matrix_t temp_matrix;

    // メインステートマシン
    always_ff @(posedge clk or negedge rst_n) begin
        if (!rst_n) begin
            reset_unit();
        end
        else begin
            // エラー処理
            if (|error_status) begin
                reset_unit();
                return;
            end

            // ステートマシン
            case (current_state)
                ST_IDLE:     handle_idle_state();
                ST_FETCH:    handle_fetch_state();
                ST_EXECUTE:  handle_execute_state();
                ST_WRITEBACK:handle_writeback_state();
            endcase
        end
    end

    // ユニットリセットタスク
    task reset_unit();
        ready <= 1'b1;
        done <= 1'b0;
        mem_request <= 1'b0;
        mem_op_type <= '0;
        vec_index <= '0;
        mat_row <= '0;
        mat_col <= '0;
        write_data <= '0;
        data_out <= '0;
        current_state <= ST_IDLE;
    endtask

    // アイドル状態ハンドリング
    task handle_idle_state();
        if (decode_valid) begin
            current_state <= ST_FETCH;
            ready <= 1'b0;
        end
    endtask

    // フェッチ状態ハンドリング
    task handle_fetch_state();
        case (decoded_ctrl.op_code)
            OP_LOAD: begin
                mem_request <= 1'b1;
                mem_op_type <= 4'b0001;
                vec_index <= decoded_ctrl.addr;
            end
            OP_STORE: begin
                mem_request <= 1'b1;
                mem_op_type <= 4'b0010;
                vec_index <= decoded_ctrl.addr;
                write_data <= data_in;
            end
            OP_COMPUTE: begin
                mem_request <= 1'b1;
                mem_op_type <= 4'b0100;
                temp_matrix <= matrix_in;
            end
            default: reset_unit();
        endcase
        current_state <= ST_EXECUTE;
    endtask

    // 実行状態ハンドリング
    task handle_execute_state();
        if (mem_done) begin
            case (decoded_ctrl.op_code)
                OP_LOAD: begin
                    temp_vector <= read_data;
                    current_state <= ST_WRITEBACK;
                end
                OP_STORE: begin
                    current_state <= ST_WRITEBACK;
                end
                OP_COMPUTE: begin
                    temp_vector <= read_data;
                    current_state <= ST_WRITEBACK;
                end
                default: reset_unit();
            endcase
            mem_request <= 1'b0;
        end
    endtask

    // ライトバック状態ハンドリング
    task handle_writeback_state();
        case (decoded_ctrl.op_code)
            OP_LOAD: begin
                data_out <= temp_vector;
                done <= 1'b1;
            end
            OP_COMPUTE: begin
                data_out <= compute_result(decoded_ctrl.comp_type);
                done <= 1'b1;
            end
            OP_STORE: begin
                done <= 1'b1;
            end
        endcase
        
        ready <= 1'b1;
        current_state <= ST_IDLE;
    endtask

    // 計算結果生成関数
    function vector_t compute_result(input comp_type_e comp_type);
        vector_t result;
        
        case (comp_type)
            COMP_ADD: begin
                for (int i = 0; i < VECTOR_DEPTH; i++) begin
                    result.data[i] = temp_vector.data[i] + read_data.data[i];
                end
            end
            COMP_MUL: begin
                for (int i = 0; i < VECTOR_DEPTH; i++) begin
                    logic [VECTOR_WIDTH-1:0] sum = '0;
                    for (int j = 0; j < MATRIX_DEPTH; j++) begin
                        if (temp_matrix.data[i][j][0]) begin
                            sum += temp_matrix.data[i][j][1] ? 
                                -temp_vector.data[j] : temp_vector.data[j];
                        end
                    end
                    result.data[i] = sum;
                end
            end
            COMP_TANH: begin
                for (int i = 0; i < VECTOR_DEPTH; i++) begin
                    result.data[i] = temp_vector.data[i][VECTOR_WIDTH-1] ? 
                        {1'b1, {(VECTOR_WIDTH-1){1'b0}}} :
                        {1'b0, {(VECTOR_WIDTH-1){1'b1}}};
                end
            end
            COMP_RELU: begin
                for (int i = 0; i < VECTOR_DEPTH; i++) begin
                    result.data[i] = temp_vector.data[i][VECTOR_WIDTH-1] ? 
                        '0 : temp_vector.data[i];
                end
            end
            default: result = temp_vector;
        endcase
        
        return result;
    endfunction

    // デバッグ用モニタリング
    // synthesis translate_off
    always @(posedge clk) begin
        if (current_state != ST_IDLE) begin
            $display("Unit %0d: state=%0d, op_code=%0d", 
                    unit_id, current_state, decoded_ctrl.op_code);
        end
    end
    // synthesis translate_on
endmodule