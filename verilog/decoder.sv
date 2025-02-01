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
    // デコード処理を組み合わせロジックで実行
    always_comb begin
        // デフォルト値の設定
        decoded_control = '0;
        decode_valid = 1'b1;
        error_status = 2'b00;

        // ユニットIDのデコード
        decoded_control.unit_id = encoded_control[5:4];
        
        // オペコードのデコード
        unique case (encoded_control[3:2])
            2'b00: decoded_control.op_code = OP_NOP;
            2'b01: decoded_control.op_code = OP_LOAD;
            2'b10: decoded_control.op_code = OP_STORE;
            2'b11: decoded_control.op_code = OP_COMP;
        endcase
        
        // 計算タイプのデコード
        unique case (encoded_control[1:0])
            2'b00: decoded_control.comp_type = COMP_ADD;
            2'b01: decoded_control.comp_type = COMP_MUL;
            2'b10: decoded_control.comp_type = COMP_TANH;
            2'b11: decoded_control.comp_type = COMP_RELU;
        endcase
        
        // データ制御フィールドのデコード
        decoded_control.addr = data_control[7:4];
        decoded_control.valid = data_control[3];
        decoded_control.size = data_control[2:0];

        // エラー検出ロジックの簡素化
        case (decoded_control.op_code)
            OP_NOP: begin
                // NOP命令では追加データがあればエラー
                if (|data_control) begin
                    decode_valid = 1'b0;
                    error_status = 2'b01;
                end
            end
            OP_COMP: begin
                // 計算命令では有効フラグが必要
                if (!data_control[3]) begin
                    decode_valid = 1'b0;
                    error_status = 2'b10;
                end
            end
            default: begin
                // その他の命令では特別な制約なし
            end
        endcase
    end
endmodule