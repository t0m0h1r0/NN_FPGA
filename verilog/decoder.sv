// decoder.sv
module decoder
    import accel_pkg::*;
(
    input  logic clk,
    input  logic rst_n,
    
    // 制御インターフェース
    input  ctrl_packet_t ctrl_packet,
    
    // デコード出力
    output decoded_ctrl_t decoded_ctrl,
    output logic decode_valid,
    output logic [1:0] error_status
);
    // エラー検出用の内部信号
    logic invalid_op_code;
    logic invalid_config;

    // デコードロジック
    always_comb begin
        // デフォルト値の設定
        decoded_ctrl = '0;
        decode_valid = 1'b1;
        error_status = 2'b00;
        invalid_op_code = 1'b0;
        invalid_config = 1'b0;

        // ユニットIDのデコード
        decoded_ctrl.unit_id = ctrl_packet.ctrl[5:4];
        
        // オペコードのデコード
        unique case (ctrl_packet.ctrl[3:2])
            2'b00: decoded_ctrl.op_code = OP_NOP;
            2'b01: decoded_ctrl.op_code = OP_LOAD;
            2'b10: decoded_ctrl.op_code = OP_STORE;
            2'b11: decoded_ctrl.op_code = OP_COMPUTE;
            default: invalid_op_code = 1'b1;
        endcase
        
        // 計算タイプのデコード
        unique case (ctrl_packet.ctrl[1:0])
            2'b00: decoded_ctrl.comp_type = COMP_ADD;
            2'b01: decoded_ctrl.comp_type = COMP_MUL;
            2'b10: decoded_ctrl.comp_type = COMP_TANH;
            2'b11: decoded_ctrl.comp_type = COMP_RELU;
            default: invalid_op_code = 1'b1;
        endcase
        
        // 構成情報のデコード
        decoded_ctrl.addr = ctrl_packet.config[7:4];
        decoded_ctrl.valid = ctrl_packet.config[3];
        decoded_ctrl.size = ctrl_packet.config[2:0];

        // エラー検出
        case (decoded_ctrl.op_code)
            OP_NOP: begin
                // NOP命令では追加データは許可されない
                if (|ctrl_packet.config) begin
                    invalid_config = 1'b1;
                end
            end
            OP_COMPUTE: begin
                // 計算命令では有効フラグが必要
                if (!ctrl_packet.config[3]) begin
                    invalid_config = 1'b1;
                end
            end
        endcase

        // エラーステータスの設定
        if (invalid_op_code) begin
            error_status = 2'b10;  // オペコードエラー
            decode_valid = 1'b0;
        end
        
        if (invalid_config) begin
            error_status = 2'b01;  // 構成情報エラー
            decode_valid = 1'b0;
        end
    end

    // デバッグ用ログ
    // synthesis translate_off
    always @(posedge clk) begin
        if (!decode_valid) begin
            $display("デコードエラー: ctrl_packet=0x%h, error_status=0x%h", 
                     ctrl_packet, error_status);
        end
    end
    // synthesis translate_on
endmodule