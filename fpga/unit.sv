// unit.sv
module processing_unit
    import accel_pkg::*;
(
    input  logic clk,
    input  logic rst_n,
    input  logic [1:0] unit_id,
    input  ctrl_packet_t control,
    output logic ready,
    output logic done,
    
    // ユニット間接続用
    input  data_t unit_data_in [UNIT_COUNT],
    output data_t unit_data_out,
    
    input  data_t data_in,
    output data_t data_out
);

    // 内部状態
    typedef enum logic [2:0] {
        ST_IDLE,
        ST_EXECUTE,
        ST_WRITEBACK
    } unit_state_e;

    // 内部信号
    data_t src_data;
    unit_state_e current_state;
    decoded_ctrl_t decoded_ctrl;
    q1_31_t vector_q31 [DATA_DEPTH];

    // ステート遷移と処理ロジック
    always_ff @(posedge clk or negedge rst_n) begin
        if (!rst_n) begin
            reset_unit();
        end else begin
            unique case (current_state)
                ST_IDLE:     handle_idle_state();
                ST_EXECUTE:  handle_execute_state();
                ST_WRITEBACK: handle_writeback_state();
                default:     reset_unit();
            endcase
        end
    end

    // アイドル状態ハンドリング
    task handle_idle_state();
        decoded_ctrl = control;
        current_state <= ST_EXECUTE;
    endtask

    // 計算ステート
    task handle_execute_state();
        unique case (decoded_ctrl.op_code)
            OP_COMPUTE: perform_computation();
            OP_COPY: perform_copy();
            OP_ADD_VEC: perform_vector_addition();
            default: reset_unit();
        endcase
        current_state <= ST_WRITEBACK;
    endtask

    // 計算処理
    task perform_computation();
        for (int i = 0; i < DATA_DEPTH; i++) begin
            q1_31_t result;
            unique case (decoded_ctrl.comp_type)
                COMP_ADD: result = add_q31(data_in.vector.data[i], 32'h3F800000);  // 1.0
                COMP_MUL: result = mul_q31(data_in.vector.data[i], 32'h40000000);  // 2.0
                COMP_TANH: result = tanh_q31(data_in.vector.data[i]);
                COMP_RELU: result = relu_q31(data_in.vector.data[i]);
                default: result = data_in.vector.data[i];
            endcase
            data_out.vector.data[i] = result;
        end
    endtask

    // ベクトルコピー
    task perform_copy();
        src_data = unit_data_in[decoded_ctrl.src_unit_id];
        data_out.vector = src_data.vector;
    endtask

    // ベクトル加算
    task perform_vector_addition();
        src_data = unit_data_in[decoded_ctrl.src_unit_id];
        for (int i = 0; i < DATA_DEPTH; i++) begin
            data_out.vector.data[i] = add_q31(
                data_in.vector.data[i], 
                src_data.vector.data[i]
            );
        end
    endtask

    // Q1.31加算
    function automatic q1_31_t add_q31(
        input q1_31_t a, 
        input q1_31_t b
    );
        logic signed [32:0] sum;
        sum = {a.sign, a.value} + {b.sign, b.value};
        return '{
            sign: sum[32],
            value: sum[31:1]
        };
    endfunction

    // Q1.31乗算
    function automatic q1_31_t mul_q31(
        input q1_31_t a, 
        input q1_31_t b
    );
        logic signed [62:0] product;
        product = {a.sign, a.value} * {b.sign, b.value};
        return '{
            sign: product[62],
            value: product[61:31]
        };
    endfunction

    // Q1.31 ReLU
    function automatic q1_31_t relu_q31(
        input q1_31_t x
    );
        return x.sign ? '{sign: 1'b0, value: '0} : x;
    endfunction

    // Q1.31 Tanh近似
    function automatic q1_31_t tanh_q31(
        input q1_31_t x
    );
        // 簡易的な双曲線正接の近似
        return x.sign ? 
            '{sign: 1'b1, value: 31'h40000000} :  // -1に近い値
            '{sign: 1'b0, value: 31'h40000000};   // 1に近い値
    endfunction

    // ライトバックステート
    task handle_writeback_state();
        unit_data_out = data_out;
        current_state <= ST_IDLE;
        done <= 1'b1;
    endtask

    // ユニットリセット
    task reset_unit();
        current_state <= ST_IDLE;
        ready <= 1'b1;
        done <= 1'b0;
        data_out <= '0;
        unit_data_out <= '0;
    endtask
endmodule