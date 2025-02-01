// decoder.sv - 主要な変更部分

module decoder
    import accel_pkg::*;
(
    input  logic clk,
    input  logic rst_n,
    input  ctrl_packet_t ctrl_packet,
    output decoded_ctrl_t decoded_ctrl,
    output logic decode_valid,
    output logic [1:0] error_status
);

    // デコードロジック（更新）
    always_comb begin
        // デフォルト値の設定
        decoded_ctrl = '0;
        decode_valid = 1'b1;
        error_status = 2'b00;

        // ユニットIDのデコード
        decoded_ctrl.unit_id = ctrl_packet.unit_id;
        decoded_ctrl.src_unit_id = ctrl_packet.src_unit_id;
        
        // オペコードのデコード（更新）
        unique case (ctrl_packet.ctrl[5:3])
            3'b000: decoded_ctrl.op_code = OP_NOP;
            3'b001: decoded_ctrl.op_code = OP_LOAD;
            3'b010: decoded_ctrl.op_code = OP_STORE;
            3'b011: decoded_ctrl.op_code = OP_COMPUTE;
            3'b100: decoded_ctrl.op_code = OP_COPY;
            3'b101: decoded_ctrl.op_code = OP_ADD_VEC;
            default: begin
                error_status[1] = 1'b1;
                decode_valid = 1'b0;
            end
        endcase
        
        // 計算タイプのデコード（既存）
        unique case (ctrl_packet.ctrl[2:1])
            2'b00: decoded_ctrl.comp_type = COMP_ADD;
            2'b01: decoded_ctrl.comp_type = COMP_MUL;
            2'b10: decoded_ctrl.comp_type = COMP_TANH;
            2'b11: decoded_ctrl.comp_type = COMP_RELU;
            default: begin
                error_status[1] = 1'b1;
                decode_valid = 1'b0;
            end
        endcase

        // 追加：ソースユニットIDの検証
        if ((decoded_ctrl.op_code == OP_COPY || 
             decoded_ctrl.op_code == OP_ADD_VEC) &&
            decoded_ctrl.src_unit_id >= UNIT_COUNT) begin
            error_status[0] = 1'b1;
            decode_valid = 1'b0;
        end
    end

endmodule