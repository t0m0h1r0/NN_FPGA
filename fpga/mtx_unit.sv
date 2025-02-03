`ifndef MTX_UNIT_MODULE
`define MTX_UNIT_MODULE

`include "mtx_types_pkg.sv"

module mtx_unit(
    input logic clk,
    input logic rst_n,
    
    // VLIW命令インターフェース
    input mtx_types::vliw_inst_t vliw_inst,
    input mtx_types::mv_t in,
    
    // 共有メモリアクセス
    input mtx_types::mv_t global_shared_mem_in,
    output mtx_types::mv_t global_shared_mem_out,
    
    // 出力
    output mtx_types::mv_t out,
    output mtx_types::status_t st
);
    import mtx_types::*;

    // レジスタ
    mv_t V0, V1, M0;
    status_t status;

    // ReLU計算関数
    function automatic mv_t relu_activation(input mv_t input_vec);
        mv_t result = input_vec;
        for (int i = 0; i < V; i++) begin
            result.vec.vec[i] = (input_vec.vec.vec[i] > 0) ? 
                input_vec.vec.vec[i] : '0;
        end
        return result;
    endfunction

    // Hard Tanh計算関数
    function automatic mv_t hard_tanh_activation(input mv_t input_vec);
        mv_t result = input_vec;
        logic signed [31:0] min_val = -32'sh1;  // -1
        logic signed [31:0] max_val = 32'sh1;   // +1
        
        for (int i = 0; i < V; i++) begin
            if (input_vec.vec.vec[i] < min_val)
                result.vec.vec[i] = min_val;
            else if (input_vec.vec.vec[i] > max_val)
                result.vec.vec[i] = max_val;
            else
                result.vec.vec[i] = input_vec.vec.vec[i];
        end
        return result;
    endfunction

    // ベクトル二乗計算関数
    function automatic mv_t vector_square(input mv_t input_vec);
        mv_t result;
        for (int i = 0; i < V; i++) begin
            // 二乗計算と飽和
            logic signed [63:0] squared = 
                $signed(input_vec.vec.vec[i]) * $signed(input_vec.vec.vec[i]);
            result.vec.vec[i] = sat(squared);
        end
        return result;
    endfunction

    // 行列-ベクトル乗算の内部関数
    function automatic mv_t matrix_vector_mul(
        input mv_t matrix,
        input mv_t vector
    );
        mv_t result;
        
        for (int r = 0; r < R; r++) begin
            logic signed [63:0] sum = '0;
            
            for (int c = 0; c < C; c++) begin
                val3_t val = matrix.mtx.data3[r][c];
                q31_t mult = mul3(val, vector.vec.vec[c]);
                sum += mult;
            end
            
            result.vec.vec[r] = sat(sum);
        end
        
        return result;
    endfunction

    always_ff @(posedge clk or negedge rst_n) begin
        if (!rst_n) begin
            // 初期化
            V0 <= '0;
            V1 <= '0;
            M0 <= '0;
            out <= '0;
            global_shared_mem_out <= '0;
            status <= '0;
        end else begin
            // デフォルトの状態リセット
            status <= '0;
            global_shared_mem_out <= '0;

            // 4段の命令を順次実行
            for (int i = 0; i < 4; i++) begin
                op_t current_op;
                
                // VLIWワードから現在の命令を選択
                unique case(i)
                    0: current_op = vliw_inst.op1;
                    1: current_op = vliw_inst.op2;
                    2: current_op = vliw_inst.op3;
                    3: current_op = vliw_inst.op4;
                endcase

                // 命令実行
                unique case(current_op)
                    NOP: begin
                        // 何も行わない
                    end

                    // ロード命令
                    LD_V0: V0 <= in;
                    LD_V1: V1 <= in;
                    LD_M0: M0 <= in;

                    // ストア命令
                    ST_V0: out <= V0;
                    ST_V1: out <= V1;
                    ST_M0: out <= M0;

                    // ゼロ初期化
                    ZERO_V0: V0 <= '0;
                    ZERO_V1: V1 <= '0;
                    ZERO_M0: M0 <= '0;

                    // メモリ関連命令
                    PUSH_V0: global_shared_mem_out <= V0;
                    POP_V1: V1 <= global_shared_mem_in;
                    POP_V0: V0 <= global_shared_mem_in; // 【新規追加】

                    // 行列-ベクトル乗算
                    MVMUL: begin
                        V0 <= matrix_vector_mul(M0, V0);
                        status.zero <= (V0.vec.vec == '0);
                    end

                    // ベクトル演算
                    VADD_01: begin
                        for (int j = 0; j < V; j++) begin
                            logic signed [33:0] sum = 
                                $signed({1'b0, V0.vec.vec[j]}) + 
                                $signed({1'b0, V1.vec.vec[j]});
                            
                            V0.vec.vec[j] <= sat(sum);
                            status.of |= (sum > 34'sh7FFFFFFF || sum < -34'sh7FFFFFFF);
                        end
                        status.zero <= (V0.vec.vec == '0);
                    end

                    VSUB_01: begin
                        for (int j = 0; j < V; j++) begin
                            logic signed [33:0] diff = 
                                $signed({1'b0, V0.vec.vec[j]}) - 
                                $signed({1'b0, V1.vec.vec[j]});
                            
                            V0.vec.vec[j] <= sat(diff);
                            status.of |= (diff > 34'sh7FFFFFFF || diff < -34'sh7FFFFFFF);
                        end
                        status.zero <= (V0.vec.vec == '0);
                    end

                    // 活性化関数
                    VRELU: begin
                        V0 <= relu_activation(V0);
                        status.zero <= (V0.vec.vec == '0);
                    end

                    VHTANH: begin
                        V0 <= hard_tanh_activation(V0);
                        status.zero <= (V0.vec.vec == '0);
                    end

                    VSQR: begin
                        V0 <= vector_square(V0);
                        status.zero <= (V0.vec.vec == '0);
                    end

                    default: begin
                        status.inv <= 1'b1;
                    end
                endcase
            end
        end
    end

    // 出力割り当て
    assign st = status;
endmodule

`endif