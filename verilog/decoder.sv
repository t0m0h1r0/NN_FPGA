// decoder.sv
module optimized_decoder
    import accel_pkg::*;
(
    input  logic clk,
    input  logic rst_n,
    
    // 最適化された制御インターフェース
    input  logic [5:0] encoded_control,    // [5:4]:unit_id, [3:2]:op_code, [1:0]:comp_type
    input  logic [7:0] data_control,       // データ制御用の追加フィールド
    
    // デコード後の制御信号
    output control_signal_t decoded_control,
    output logic decode_valid,
    output logic [1:0] error_status
);
    // エラー検出用の内部信号
    logic invalid_op_code;
    logic invalid_data_control;

    // デコード処理と検証
    always_comb begin
        // デフォルト値の設定
        decoded_control = '0;
        decode_valid = 1'b1;
        error_status = 2'b00;
        invalid_op_code = 1'b0;
        invalid_data_control = 1'b0;

        // ユニットIDのデコード
        decoded_control.unit_id = encoded_control[5:4];
        
        // オペコードのデコード
        unique case (encoded_control[3:2])
            2'b00: begin
                decoded_control.op_code = OP_NOP;
                // NOP命令では追加データは許可されない
                if (|data_control) begin
                    invalid_data_control = 1'b1;
                end
            end
            2'b01: decoded_control.op_code = OP_LOAD;
            2'b10: decoded_control.op_code = OP_STORE;
            2'b11: begin
                decoded_control.op_code = OP_COMP;
                // 計算命令では有効フラグが必要
                if (!data_control[3]) begin
                    invalid_data_control = 1'b1;
                end
            end
            default: begin
                // 不正なオペコード
                invalid_op_code = 1'b1;
            end
        endcase
        
        // 計算タイプのデコード
        unique case (encoded_control[1:0])
            2'b00: decoded_control.comp_type = COMP_ADD;
            2'b01: decoded_control.comp_type = COMP_MUL;
            2'b10: decoded_control.comp_type = COMP_TANH;
            2'b11: decoded_control.comp_type = COMP_RELU;
            default: begin
                // 不正な計算タイプ（通常は発生しない）
                invalid_op_code = 1'b1;
            end
        endcase
        
        // データ制御フィールドのデコード
        decoded_control.addr = data_control[7:4];
        decoded_control.valid = data_control[3];
        decoded_control.size = data_control[2:0];

        // エラーステータスの設定
        if (invalid_op_code) begin
            error_status = 2'b10;  // オペコードエラー
            decode_valid = 1'b0;
        end
        
        if (invalid_data_control) begin
            error_status = 2'b01;  // データ制御エラー
            decode_valid = 1'b0;
        end
    end

    // デバッグ用ログ
    // synthesis translate_off
    always @(posedge clk) begin
        if (!decode_valid) begin
            $display("デコードエラー: encoded_control=0x%h, data_control=0x%h, error_status=0x%h", 
                     encoded_control, data_control, error_status);
        end
    end
    // synthesis translate_on
endmodule