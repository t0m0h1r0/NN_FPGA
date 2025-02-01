// shared_compute_unit.sv
module shared_compute_unit
    import accel_pkg::*;
(
    input  logic clk,
    input  logic rst_n,
    
    // 制御インターフェース
    input  logic [1:0] unit_id,
    input  logic request,
    output logic ready,
    output logic done,
    
    // データインターフェース
    input  comp_type_e comp_type,
    input  data_t data_in,
    output data_t result
);
    // 計算ステージ
    typedef enum logic [1:0] {
        ST_IDLE,
        ST_COMPUTE,
        ST_COMPLETE
    } compute_state_e;

    // 内部状態
    compute_state_e current_state;
    logic [1:0] current_unit;
    logic [4:0] compute_counter;

    // ステータス制御
    always_ff @(posedge clk or negedge rst_n) begin
        if (!rst_n) begin
            reset_unit();
        end
        else begin
            // ステートマシン
            case (current_state)
                ST_IDLE:     handle_idle_state();
                ST_COMPUTE:  handle_compute_state();
                ST_COMPLETE: handle_complete_state();
            endcase
        end
    end

    // ユニットリセット
    task reset_unit();
        ready <= 1'b1;
        done <= 1'b0;
        current_state <= ST_IDLE;
        current_unit <= 2'b00;
        compute_counter <= '0;
    endtask

    // アイドル状態ハンドリング
    task handle_idle_state();
        if (request) begin
            current_unit <= unit_id;
            current_state <= ST_COMPUTE;
            ready <= 1'b0;
            compute_counter <= '0;
        end
    endtask

    // 計算状態ハンドリング
    task handle_compute_state();
        // 計算タイプに応じた処理
        unique case (comp_type)
            COMP_ADD:  compute_addition();
            COMP_MUL:  compute_matrix_multiplication();
            COMP_TANH: compute_tanh_activation();
            COMP_RELU: compute_relu_activation();
        endcase

        // カウンタ更新と状態遷移
        compute_counter <= compute_counter + 1;
        if (compute_counter == DATA_DEPTH - 1) begin
            current_state <= ST_COMPLETE;
        end
    endtask

    // 加算計算
    task compute_addition();
        for (int i = 0; i < DATA_DEPTH; i++) begin
            result.vector.data[i] = data_in.vector.data[i] + data_in.matrix.data[i][0];
        end
    endtask

    // 行列乗算
    task compute_matrix_multiplication();
        for (int i = 0; i < DATA_DEPTH; i++) begin
            logic [VECTOR_WIDTH-1:0] sum = '0;
            for (int j = 0; j < DATA_DEPTH; j++) begin
                if (data_in.matrix.data[i][j][0]) begin
                    sum += data_in.matrix.data[i][j][1] ?
                        -data_in.vector.data[j] : data_in.vector.data[j];
                end
            end
            result.vector.data[i] = sum;
        end
    endtask

    // Tanh活性化
    task compute_tanh_activation();
        for (int i = 0; i < DATA_DEPTH; i++) begin
            result.vector.data[i] = data_in.vector.data[i][VECTOR_WIDTH-1] ?
                {1'b1, {(VECTOR_WIDTH-1){1'b0}}} :
                {1'b0, {(VECTOR_WIDTH-1){1'b1}}};
        end
    endtask

    // ReLU活性化
    task compute_relu_activation();
        for (int i = 0; i < DATA_DEPTH; i++) begin
            result.vector.data[i] = data_in.vector.data[i][VECTOR_WIDTH-1] ?
                '0 : data_in.vector.data[i];
        end
    endtask

    // 完了状態ハンドリング
    task handle_complete_state();
        done <= 1'b1;
        ready <= 1'b1;
        current_state <= ST_IDLE;
    endtask

endmodule