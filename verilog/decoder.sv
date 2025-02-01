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
    output logic [1:0] unit_id,
    output operation_code_t op_code,
    output computation_type_t comp_type,
    output logic [3:0] data_addr,
    output logic data_valid,
    output logic [2:0] transfer_size
);
    // エンコードされた命令のフィールド抽出
    assign unit_id = encoded_control[5:4];
    
    // オペコードのデコード
    always_comb begin
        case (encoded_control[3:2])
            2'b00: op_code = OP_NOP;
            2'b01: op_code = OP_LOAD;
            2'b10: op_code = OP_STORE;
            2'b11: op_code = OP_COMP;
        endcase
    end
    
    // 計算タイプのデコード
    always_comb begin
        case (encoded_control[1:0])
            2'b00: comp_type = COMP_ADD;
            2'b01: comp_type = COMP_MUL;
            2'b10: comp_type = COMP_TANH;
            2'b11: comp_type = COMP_RELU;
        endcase
    end
    
    // データ制御フィールドのデコード
    always_comb begin
        data_addr = data_control[7:4];
        data_valid = data_control[3];
        transfer_size = data_control[2:0];
    end

endmodule

// 命令エンコーダ（テスト用）
module instruction_encoder
    import accel_pkg::*;
(
    // 元の制御パケット形式
    input  control_packet_t orig_control,
    
    // エンコードされた出力
    output logic [5:0] encoded_control,
    output logic [7:0] data_control
);
    // 制御信号のエンコード
    assign encoded_control = {
        orig_control.unit_id[1:0],     // [5:4]
        orig_control.op_code[1:0],     // [3:2]
        orig_control.comp_type[1:0]    // [1:0]
    };
    
    // データ制御信号のエンコード
    assign data_control = {
        orig_control.addr[3:0],        // [7:4]
        orig_control.valid,            // [3]
        orig_control.size[2:0]         // [2:0]
    };

endmodule

// デコーダユニット（最上位モジュール）
module decoder_unit
    import accel_pkg::*;
(
    input  logic clk,
    input  logic rst_n,
    
    // 制御インターフェース
    input  logic [13:0] instruction_packet,  // エンコードされた命令パケット
    output control_signal_t decoded_control,  // デコードされた制御信号
    
    // ステータス出力
    output logic decode_valid,
    output logic [1:0] error_status
);
    // 内部信号
    logic [5:0] encoded_control;
    logic [7:0] data_control;
    logic decode_error;
    
    // パケットの分割
    assign encoded_control = instruction_packet[13:8];
    assign data_control = instruction_packet[7:0];
    
    // デコーダインスタンス
    optimized_decoder u_decoder (
        .clk(clk),
        .rst_n(rst_n),
        .encoded_control(encoded_control),
        .data_control(data_control),
        .unit_id(decoded_control.unit_id),
        .op_code(decoded_control.op_code),
        .comp_type(decoded_control.comp_type),
        .data_addr(decoded_control.addr),
        .data_valid(decoded_control.valid),
        .transfer_size(decoded_control.size)
    );
    
    // エラー検出ロジック
    always_comb begin
        decode_error = 1'b0;
        case (encoded_control[3:2])
            2'b00: begin // NOP
                if (|data_control) decode_error = 1'b1;
            end
            2'b11: begin // COMP
                if (!data_control[3]) decode_error = 1'b1;
            end
            default: decode_error = 1'b0;
        endcase
    end
    
    // ステータス更新
    always_ff @(posedge clk or negedge rst_n) begin
        if (!rst_n) begin
            decode_valid <= 1'b0;
            error_status <= 2'b00;
        end
        else begin
            decode_valid <= !decode_error;
            error_status <= {decode_error, |encoded_control};
        end
    end

    // synthesis translate_off
    // デバッグ用モニタリング
    always @(posedge clk) begin
        if (decode_error) begin
            $display("Decode Error: instruction=0x%h", instruction_packet);
        end
    end
    // synthesis translate_on

endmodule