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
    input  computation_type_t comp_type,
    input  vector_data_t vector_a,
    input  vector_data_t vector_b,
    input  matrix_data_t matrix_in,
    output vector_data_t result
);
    // 内部状態と制御信号
    logic [1:0] current_unit;
    logic processing;
    logic [4:0] compute_counter;
    
    // ステータス制御モジュール
    status_control u_status (
        .clk(clk),
        .rst_n(rst_n),
        .start(processing),
        .max_count(5'd16),  // VECTOR_DEPTH分のカウント
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
        .result(result.data[compute_counter])
    );

    // 行列演算用の内部ロジック
    logic [VECTOR_WIDTH-1:0] matrix_product;
    logic matrix_valid;

    // メインステートマシン
    always_ff @(posedge clk or negedge rst_n) begin
        if (!rst_n) begin
            reset_unit();
        end
        else begin
            // 新しい演算要求の処理
            if (!processing && request) begin
                handle_new_request();
            end

            // 行列乗算の特別な処理
            if (comp_type == COMP_MUL && processing) begin
                handle_matrix_multiplication();
            end
        end
    end

    // ユニットのリセット
    task reset_unit();
        ready <= 1'b1;
        processing <= 1'b0;
        current_unit <= 2'b00;
        matrix_valid <= 1'b0;
    endtask

    // 新しい演算要求の処理
    task handle_new_request();
        current_unit <= unit_id;
        processing <= 1'b1;
        ready <= 1'b0;
    endtask

    // 行列乗算の処理
    task handle_matrix_multiplication();
        matrix_product <= '0;
        matrix_valid <= 1'b1;
        
        for (int j = 0; j < MATRIX_DEPTH; j++) begin
            if (matrix_in.data[compute_counter][j][0]) begin
                matrix_product <= matrix_product + 
                    (matrix_in.data[compute_counter][j][1] ? 
                     -vector_a.data[j] : vector_a.data[j]);
            end
        end
        
        // 最終的な結果の設定
        if (done) begin
            result.data[compute_counter] <= matrix_product;
        end
    endtask

    // 結果の選択と出力
    always_comb begin
        // 演算完了時かつ要求元ユニットが一致する場合のみ結果を出力
        result = (done && current_unit == unit_id) ? result : '0;
    end
endmodule