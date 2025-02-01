// shared_compute_unit.sv の更新
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
    // 1s.31形式の固定小数点数を使用
    fixed_point_1s31_t fixed_input_vector [DATA_DEPTH];
    fixed_point_1s31_t fixed_result_vector [DATA_DEPTH];

    // データ変換ステージ
    always_comb begin
        // 入力データを1s.31形式に変換
        for (int i = 0; i < DATA_DEPTH; i++) begin
            fixed_input_vector[i] = float_to_fixed_point_1s31(data_in.vector.data[i]);
        end
    end

    // 計算ステージ（固定小数点演算）
    always_comb begin
        unique case (comp_type)
            COMP_ADD: compute_fixed_addition();
            COMP_MUL: compute_fixed_multiplication();
            COMP_TANH: compute_fixed_tanh();
            COMP_RELU: compute_fixed_relu();
            default: reset_computation();
        endcase
    end

    // 固定小数点加算
    task compute_fixed_addition();
        for (int i = 0; i < DATA_DEPTH; i++) begin
            fixed_result_vector[i] = add_fixed_point(
                fixed_input_vector[i], 
                float_to_fixed_point_1s31(32'h3F800000)  // 1.0のIEEE 754表現
            );
        end
    endtask

    // 固定小数点乗算
    task compute_fixed_multiplication();
        for (int i = 0; i < DATA_DEPTH; i++) begin
            fixed_result_vector[i] = multiply_fixed_point(
                fixed_input_vector[i], 
                float_to_fixed_point_1s31(32'h40000000)  // 2.0のIEEE 754表現
            );
        end
    endtask

    // 固定小数点Tanh近似
    task compute_fixed_tanh();
        for (int i = 0; i < DATA_DEPTH; i++) begin
            fixed_result_vector[i] = approximate_tanh(fixed_input_vector[i]);
        end
    endtask

    // 固定小数点ReLU
    task compute_fixed_relu();
        for (int i = 0; i < DATA_DEPTH; i++) begin
            fixed_result_vector[i] = relu_fixed_point(fixed_input_vector[i]);
        end
    endtask

    // 結果の変換
    always_comb begin
        for (int i = 0; i < DATA_DEPTH; i++) begin
            result.vector.data[i] = fixed_point_1s31_to_float(fixed_result_vector[i]);
        end
    end

    // 固定小数点加算関数
    function automatic fixed_point_1s31_t add_fixed_point(
        input fixed_point_1s31_t a, 
        input fixed_point_1s31_t b
    );
        logic signed [32:0] sum;
        sum = {a.sign, a.value} + {b.sign, b.value};
        return '{
            sign: sum[32],
            value: sum[31:1]
        };
    endfunction

    // 固定小数点乗算関数
    function automatic fixed_point_1s31_t multiply_fixed_point(
        input fixed_point_1s31_t a, 
        input fixed_point_1s31_t b
    );
        logic signed [62:0] product;
        product = {a.sign, a.value} * {b.sign, b.value};
        return '{
            sign: product[62],
            value: product[61:31]
        };
    endfunction

    // Tanh近似関数（固定小数点）
    function automatic fixed_point_1s31_t approximate_tanh(
        input fixed_point_1s31_t x
    );
        // 単純な双曲線正接の近似実装
        if (x.sign && x.value == 0) begin
            return '{sign: 1'b1, value: '0};  // 負のゼロ
        end else if (!x.sign && x.value == 0) begin
            return '{sign: 1'b0, value: '0};  // 正のゼロ
        end else begin
            // 簡易的な双曲線正接近似
            return x.sign ? 
                '{sign: 1'b1, value: 31'h40000000} :  // -1に近い値
                '{sign: 1'b0, value: 31'h40000000};   // 1に近い値
        end
    endfunction

    // ReLU関数（固定小数点）
    function automatic fixed_point_1s31_t relu_fixed_point(
        input fixed_point_1s31_t x
    );
        // 負の値は0に、正の値はそのまま
        return x.sign ? '{sign: 1'b0, value: '0} : x;
    endfunction

    // リセット時の初期化
    task reset_computation();
        for (int i = 0; i < DATA_DEPTH; i++) begin
            fixed_result_vector[i] = '{sign: 1'b0, value: '0};
        end
    endtask
endmodule