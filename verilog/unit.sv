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
    // 内部状態と制御信号
    control_signal_t decoded_control;
    logic decode_valid;
    logic [1:0] error_status;

    // デコーダインスタンス
    optimized_decoder u_decoder (
        .clk(clk),
        .rst_n(rst_n),
        .encoded_control({control.encoded_control, control.data_control}),
        .decoded_control(decoded_control),
        .decode_valid(decode_valid),
        .error_status(error_status)
    );

    // 内部状態と制御レジスタ
    logic compute_active;
    vector_data_t computed_vector;
    matrix_data_t stored_matrix;

    // 状態遷移と制御ロジック
    always_ff @(posedge clk or negedge rst_n) begin
        if (!rst_n) begin
            // リセット時の初期化
            reset_unit();
        end
        else begin
            // エラー処理
            if (|error_status) begin
                reset_unit();
                return;
            end

            // メイン状態遷移
            case (decoded_control.op_code)
                OP_NOP: handle_nop_operation();
                OP_LOAD: handle_load_operation();
                OP_STORE: handle_store_operation();
                OP_COMP: handle_compute_operation();
            endcase
        end
    end

    // タスク：ユニットのリセット
    task reset_unit();
        ready <= 1'b1;
        done <= 1'b0;
        mem_request <= 1'b0;
        compute_active <= 1'b0;
        data_out <= '0;
    endtask

    // タスク：NOP操作の処理
    task handle_nop_operation();
        ready <= 1'b1;
        done <= 1'b0;
    endtask

    // タスク：ロード操作の処理
    task handle_load_operation();
        if (decode_valid && !mem_request) begin
            ready <= 1'b0;
            mem_request <= 1'b1;
            mem_op_type <= 4'b0001;  // Load操作
            vec_index <= decoded_control.addr;
        end
        
        if (mem_done) begin
            data_out <= read_data;
            mem_request <= 1'b0;
            done <= 1'b1;
            ready <= 1'b1;
        end
    endtask

    // タスク：ストア操作の処理
    task handle_store_operation();
        if (decode_valid && !mem_request) begin
            ready <= 1'b0;
            mem_request <= 1'b1;
            mem_op_type <= 4'b0010;  // Store操作
            vec_index <= decoded_control.addr;
            write_data <= data_in;
        end
        
        if (mem_done) begin
            mem_request <= 1'b0;
            done <= 1'b1;
            ready <= 1'b1;
        end
    endtask

    // タスク：計算操作の処理
    task handle_compute_operation();
        if (decode_valid && !compute_active) begin
            ready <= 1'b0;
            compute_active <= 1'b1;
            stored_matrix <= matrix_in;
            mem_request <= 1'b1;
            mem_op_type <= 4'b0100;  // Compute操作
        end
        
        if (mem_done) begin
            computed_vector <= read_data;
            mem_request <= 1'b0;
            
            // 計算タイプに応じた演算
            case (decoded_control.comp_type)
                COMP_ADD: perform_addition();
                COMP_MUL: perform_matrix_multiplication();
                COMP_TANH: perform_tanh_activation();
                COMP_RELU: perform_relu_activation();
            endcase

            compute_active <= 1'b0;
            done <= 1'b1;
            ready <= 1'b1;
        end
    endtask

    // 各演算タスク
    task perform_addition();
        for (int i = 0; i < VECTOR_DEPTH; i++) begin
            data_out.data[i] <= computed_vector.data[i] + read_data.data[i];
        end
    endtask

    task perform_matrix_multiplication();
        for (int i = 0; i < VECTOR_DEPTH; i++) begin
            logic [VECTOR_WIDTH-1:0] sum = '0;
            for (int j = 0; j < MATRIX_DEPTH; j++) begin
                if (stored_matrix.data[i][j][0]) begin
                    sum += stored_matrix.data[i][j][1] ? 
                        -computed_vector.data[j] : computed_vector.data[j];
                end
            end
            data_out.data[i] <= sum;
        end
    endtask

    task perform_tanh_activation();
        for (int i = 0; i < VECTOR_DEPTH; i++) begin
            data_out.data[i] <= computed_vector.data[i][VECTOR_WIDTH-1] ? 
                {1'b1, {(VECTOR_WIDTH-1){1'b0}}} :
                {1'b0, {(VECTOR_WIDTH-1){1'b1}}};
        end
    endtask

    task perform_relu_activation();
        for (int i = 0; i < VECTOR_DEPTH; i++) begin
            data_out.data[i] <= computed_vector.data[i][VECTOR_WIDTH-1] ? 
                '0 : computed_vector.data[i];
        end
    endtask

    // デバッグ用モニタリング
    // synthesis translate_off
    always @(posedge clk) begin
        if (|error_status) begin
            $display("Unit %0d Error: state=%0d, error=0x%h", 
                    unit_id, decoded_control.op_code, error_status);
        end
    end
    // synthesis translate_on
endmodule