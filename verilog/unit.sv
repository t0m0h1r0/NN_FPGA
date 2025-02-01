// unit.sv - 主要な変更部分

module processing_unit
    import accel_pkg::*;
(
    // 既存のポート
    input  logic clk,
    input  logic rst_n,
    input  logic [1:0] unit_id,
    input  ctrl_packet_t control,
    output logic ready,
    output logic done,
    
    // 追加：ユニット間接続用
    input  data_t unit_data_in [UNIT_COUNT],  // 他ユニットからのデータ入力
    output data_t unit_data_out,              // 他ユニットへのデータ出力
    
    // 既存のデータインターフェース
    input  data_t data_in,
    output data_t data_out
);

    // 内部状態（更新）
    typedef enum logic [2:0] {
        ST_IDLE,
        ST_FETCH,
        ST_EXECUTE,
        ST_COPY_WAIT,  // 追加：コピー待ち状態
        ST_WRITEBACK
    } unit_state_e;

    // 内部信号（追加）
    data_t src_data;
    logic copy_ready;

    // 状態ハンドリング関数の更新
    task handle_execute_state();
        case (decoded_ctrl.op_code)
            OP_LOAD: begin
                // 既存のロード処理
            end
            
            OP_STORE: begin
                // 既存のストア処理
            end
            
            OP_COMPUTE: begin
                // 既存の計算処理
            end
            
            OP_COPY: begin
                // コピー処理の実装
                src_data = unit_data_in[decoded_ctrl.src_unit_id];
                data_out.vector = src_data.vector;
                current_state <= ST_WRITEBACK;
            end
            
            OP_ADD_VEC: begin
                // ベクトル加算の実装
                src_data = unit_data_in[decoded_ctrl.src_unit_id];
                for (int i = 0; i < DATA_DEPTH; i++) begin
                    data_out.vector.data[i] = data_in.vector.data[i] + 
                                            src_data.vector.data[i];
                end
                current_state <= ST_WRITEBACK;
            end
            
            default: reset_unit();
        endcase
    endtask

    // 出力データの更新
    always_comb begin
        unit_data_out = data_out;
    end

endmodule