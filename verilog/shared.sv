// shared_compute_unit.sv
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
    input  comp_type_e comp_type,
    input  vector_t vector_a,
    input  vector_t vector_b,
    input  matrix_t matrix_in,
    output vector_t result
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
        if (compute_counter == VECTOR_DEPTH - 1) begin
            current_state <= ST_COMPLETE;
        end
    endtask

    // 加算計算
    task compute_addition();
        result.data[compute_counter] = 
            vector_a.data[compute_counter] + vector_b.data[compute_counter];
    endtask

    // 行列乗算
    task compute_matrix_multiplication();
        logic [VECTOR_WIDTH-1:0] sum = '0;
        for (int j = 0; j < MATRIX_DEPTH; j++) begin
            if (matrix_in.data[compute_counter][j][0]) begin
                sum += matrix_in.data[compute_counter][j][1] ? 
                    -vector_a.data[j] : vector_a.data[j];
            end
        end
        result.data[compute_counter] = sum;
    endtask

    // Tanh活性化
    task compute_tanh_activation();
        result.data[compute_counter] = 
            vector_a.data[compute_counter][VECTOR_WIDTH-1] ? 
            {1'b1, {(VECTOR_WIDTH-1){1'b0}}} :
            {1'b0, {(VECTOR_WIDTH-1){1'b1}}};
    endtask

    // ReLU活性化
    task compute_relu_activation();
        result.data[compute_counter] = 
            vector_a.data[compute_counter][VECTOR_WIDTH-1] ? 
            '0 : vector_a.data[compute_counter];
    endtask

    // 完了状態ハンドリング
    task handle_complete_state();
        done <= 1'b1;
        ready <= 1'b1;
        current_state <= ST_IDLE;
    endtask

    // デバッグ用モニタリング
    // synthesis translate_off
    always @(posedge clk) begin
        if (current_state == ST_COMPUTE) begin
            $display("共有計算ユニット: unit_id=%0d, comp_type=%0d, counter=%0d", 
                     current_unit, comp_type, compute_counter);
        end
    end
    // synthesis translate_on
endmodule