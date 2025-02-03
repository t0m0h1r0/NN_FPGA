`ifndef MTX_UNIT_MODULE
`define MTX_UNIT_MODULE

module mtx_unit #(
    parameter Q = 23,      // 小数部のビット数
    parameter INT = 8      // 整数部のビット数
) (
    input logic clk,
    input logic rst_n,
    
    // VLIW命令インターフェース
    input mtx_types #(.Q(Q), .INT(INT))::vliw_inst_t vliw_inst,
    input mtx_types #(.Q(Q), .INT(INT))::mv_t in,
    
    // 共有メモリアクセス
    input mtx_types #(.Q(Q), .INT(INT))::mv_t global_shared_mem_in,
    output mtx_types #(.Q(Q), .INT(INT))::mv_t global_shared_mem_out,
    
    // 出力
    output mtx_types #(.Q(Q), .INT(INT))::mv_t out,
    output mtx_types #(.Q(Q), .INT(INT))::status_t st
);
    import mtx_types #(.Q(Q), .INT(INT))::*;

    // レジスタ
    vec_t V0, V1;     // ベクトルレジスタ
    mtx_t M0;         // 行列レジスタ
    status_t status;  // 状態レジスタ

    // ReLU計算関数
    function automatic vec_t relu_activation(input vec_t input_vec);
        vec_t result;
        for (int i = 0; i < V; i++) begin
            result.elements[i] = (input_vec.elements[i] > 0) ? 
                input_vec.elements[i] : '0;
        end
        return result;
    endfunction

    // Hard Tanh計算関数
    function automatic vec_t hard_tanh_activation(input vec_t input_vec);
        vec_t result;
        qformat_t min_val = -(1 << Q);  // -1 in Qフォーマット
        qformat_t max_val = (1 << Q);   // +1 in Qフォーマット
        
        for (int i = 0; i < V; i++) begin
            if (input_vec.elements[i] < min_val)
                result.elements[i] = min_val;
            else if (input_vec.elements[i] > max_val)
                result.elements[i] = max_val;
            else
                result.elements[i] = input_vec.elements[i];
        end
        return result;
    endfunction

    // ベクトル二乗計算関数
    function automatic vec_t vector_square(input vec_t input_vec);
        vec_t result;
        for (int i = 0; i < V; i++) begin
            // 乗算結果の上位ビットを確認してオーバーフローを検出
            logic signed [TOTAL+Q-1:0] squared = 
                $signed(input_vec.elements[i]) * $signed(input_vec.elements[i]);
            result.elements[i] = squared[TOTAL+Q-1:Q]; // Q分右シフト相当
        end
        return result;
    endfunction

    // 行列-ベクトル乗算の内部関数
    function automatic vec_t matrix_vector_mul(
        input mtx_t matrix,
        input vec_t vector,
        output logic of_flag  // オーバーフロー検出用
    );
        vec_t result;
        of_flag = 0;
        
        for (int r = 0; r < R; r++) begin
            logic signed [TOTAL+Q-1:0] sum = '0;
            
            for (int c = 0; c < C; c++) begin
                val3_t val = matrix.elements[r][c];
                qformat_t mult = mul3_qformat(val, vector.elements[c]);
                sum += mult;
            end
            
            // オーバーフロー検出
            if (sum[TOTAL+Q-1:TOTAL+Q-INT-1] != {(INT+1){sum[TOTAL+Q-1]}}) begin
                of_flag = 1;
            end
            
            // 上位ビットを破棄して代入（自動的な切り捨て）
            result.elements[r] = sum[TOTAL+Q-1:Q];
        end
        
        return result;
    endfunction

    always_ff @(posedge clk or negedge rst_n) begin
        if (!rst_n) begin
            V0 <= '0;
            V1 <= '0;
            M0 <= '0;
            out <= '0;
            global_shared_mem_out <= '0;
            status <= '0;
        end else begin
            status <= '0;
            global_shared_mem_out <= '0;

            for (int i = 0; i < 4; i++) begin
                op_t current_op;
                logic of_detected;
                
                unique case(i)
                    0: current_op = vliw_inst.op1;
                    1: current_op = vliw_inst.op2;
                    2: current_op = vliw_inst.op3;
                    3: current_op = vliw_inst.op4;
                endcase

                unique case(current_op)
                    NOP: begin end
                    
                    LD_V0: V0 <= in.vec;
                    LD_V1: V1 <= in.vec;
                    LD_M0: M0 <= in.mtx;

                    ST_V0: out.vec <= V0;
                    ST_V1: out.vec <= V1;
                    ST_M0: out.mtx <= M0;

                    ZERO_V0: V0 <= '0;
                    ZERO_V1: V1 <= '0;
                    ZERO_M0: M0 <= '0;

                    PUSH_V0: global_shared_mem_out.vec <= V0;
                    PULL_V1: V1 <= global_shared_mem_in.vec;
                    PULL_V0: V0 <= global_shared_mem_in.vec;

                    MVMUL: begin
                        V0 <= matrix_vector_mul(M0, V0, of_detected);
                        status.of <= of_detected;
                        status.zero <= (V0.elements == '0);
                    end

                    VADD_01: begin
                        for (int j = 0; j < V; j++) begin
                            logic signed [TOTAL:0] sum = 
                                $signed({1'b0, V0.elements[j]}) + 
                                $signed({1'b0, V1.elements[j]});
                            
                            V0.elements[j] <= sum[TOTAL-1:0];
                            status.of |= (V0.elements[j][TOTAL-1] != sum[TOTAL]);
                        end
                        status.zero <= (V0.elements == '0);
                    end

                    VSUB_01: begin
                        for (int j = 0; j < V; j++) begin
                            logic signed [TOTAL:0] diff = 
                                $signed({1'b0, V0.elements[j]}) - 
                                $signed({1'b0, V1.elements[j]});
                            
                            V0.elements[j] <= diff[TOTAL-1:0];
                            status.of |= (V0.elements[j][TOTAL-1] != diff[TOTAL]);
                        end
                        status.zero <= (V0.elements == '0);
                    end

                    VRELU: begin
                        V0 <= relu_activation(V0);
                        status.zero <= (V0.elements == '0);
                    end

                    VHTANH: begin
                        V0 <= hard_tanh_activation(V0);
                        status.zero <= (V0.elements == '0);
                    end

                    VSQR: begin
                        V0 <= vector_square(V0);
                        status.of <= (V0.elements == '0);  // オーバーフロー検出
                    end

                    default: begin
                        status.inv <= 1'b1;
                    end
                endcase
            end
        end
    end

    assign st = status;
endmodule

`endif