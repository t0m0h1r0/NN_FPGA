// memory_controller.sv
module memory_controller
    import accel_pkg::*;
(
    input  logic clk,
    input  logic rst_n,

    // ユニットインターフェース（4ユニット分）
    input  logic [3:0] unit_request,
    input  logic [3:0] [3:0] unit_op_type,    // 各ユニットの操作タイプ
    input  logic [3:0] [3:0] unit_vec_index,  // ベクトルインデックス
    input  logic [3:0] [3:0] unit_mat_row,    // 行列行インデックス
    input  logic [3:0] [3:0] unit_mat_col,    // 行列列インデックス
    input  vector_data_t unit_write_data [4],  // 書き込みデータ
    output logic [3:0] unit_grant,            // アクセス許可
    output vector_data_t unit_read_data [4],   // 読み出しデータ
    output logic [3:0] unit_done,             // 完了通知

    // 共有メモリインターフェース
    output logic [5:0] mem_addr_a,
    output logic mem_we_a,
    output logic [VECTOR_WIDTH-1:0] mem_wdata_a,
    input  logic [VECTOR_WIDTH-1:0] mem_rdata_a,
    output logic [7:0] mem_addr_b,
    output logic mem_we_b,
    output logic [1:0] mem_wdata_b,
    input  logic [1:0] mem_rdata_b,
    input  logic mem_busy,
    input  logic [1:0] mem_error
);
    // 内部状態定義
    typedef enum logic [2:0] {
        IDLE,
        ARBITRATE,
        READ_SETUP,
        READ_WAIT,
        WRITE_SETUP,
        WRITE_EXECUTE,
        ERROR_HANDLE,
        COMPLETE
    } ctrl_state_t;

    ctrl_state_t current_state;
    logic [1:0] selected_unit;
    logic [3:0] operation_type;
    logic [3:0] access_counter;
    logic [3:0] priority_rotate;

    // アドレス生成インスタンス
    logic [5:0] vector_addr;
    logic [7:0] matrix_addr;
    
    memory_address_generator addr_gen (
        .unit_id(selected_unit),
        .vector_index(unit_vec_index[selected_unit]),
        .matrix_row(unit_mat_row[selected_unit]),
        .matrix_col(unit_mat_col[selected_unit]),
        .vector_addr(vector_addr),
        .matrix_addr(matrix_addr)
    );

    // 優先順位に基づくユニット選択
    function automatic logic [1:0] get_next_unit;
        input logic [3:0] requests;
        input logic [3:0] priority;
        logic [3:0] masked_requests;
        logic [3:0] rotated_requests;
        logic [1:0] selected;
        
        // 優先順位でマスクされた要求を生成
        masked_requests = requests & priority;
        if (|masked_requests) begin
            // マスクされた要求から最も優先度の高いものを選択
            casez (masked_requests)
                4'b???1: selected = 2'd0;
                4'b??10: selected = 2'd1;
                4'b?100: selected = 2'd2;
                4'b1000: selected = 2'd3;
                default: selected = 2'd0;
            endcase
        end
        else begin
            // マスクされた要求がない場合は通常の要求から選択
            casez (requests)
                4'b???1: selected = 2'd0;
                4'b??10: selected = 2'd1;
                4'b?100: selected = 2'd2;
                4'b1000: selected = 2'd3;
                default: selected = 2'd0;
            endcase
        end
        return selected;
    endfunction

    // メインステートマシン
    always_ff @(posedge clk or negedge rst_n) begin
        if (!rst_n) begin
            current_state <= IDLE;
            selected_unit <= 2'b00;
            operation_type <= 4'b0000;
            access_counter <= 4'b0000;
            priority_rotate <= 4'b0001;
            unit_grant <= 4'b0000;
            unit_done <= 4'b0000;
            mem_we_a <= 1'b0;
            mem_we_b <= 1'b0;
        end
        else begin
            case (current_state)
                IDLE: begin
                    if (|unit_request) begin
                        current_state <= ARBITRATE;
                        unit_done <= 4'b0000;
                    end
                end

                ARBITRATE: begin
                    selected_unit <= get_next_unit(unit_request, priority_rotate);
                    operation_type <= unit_op_type[selected_unit];
                    access_counter <= 4'b0000;
                    // 優先順位の更新
                    priority_rotate <= {priority_rotate[2:0], priority_rotate[3]};
                    current_state <= READ_SETUP;
                end

                READ_SETUP: begin
                    mem_addr_a <= vector_addr;
                    mem_addr_b <= matrix_addr;
                    unit_grant[selected_unit] <= 1'b1;
                    mem_we_a <= 1'b0;
                    mem_we_b <= 1'b0;
                    if (!mem_busy) begin
                        current_state <= READ_WAIT;
                    end
                end

                READ_WAIT: begin
                    if (operation_type[3]) begin
                        // 書き込み操作の場合
                        current_state <= WRITE_SETUP;
                    end
                    else begin
                        // 読み出し操作の場合
                        unit_read_data[selected_unit] <= mem_rdata_a;
                        current_state <= COMPLETE;
                    end
                end

                WRITE_SETUP: begin
                    mem_we_a <= 1'b1;
                    mem_wdata_a <= unit_write_data[selected_unit].data[access_counter];
                    current_state <= WRITE_EXECUTE;
                end

                WRITE_EXECUTE: begin
                    if (!mem_busy) begin
                        if (access_counter == 4'hF) begin
                            current_state <= COMPLETE;
                        end
                        else begin
                            access_counter <= access_counter + 1;
                            current_state <= WRITE_SETUP;
                        end
                    end
                end

                ERROR_HANDLE: begin
                    // エラー処理
                    unit_grant <= 4'b0000;
                    current_state <= IDLE;
                end

                COMPLETE: begin
                    unit_grant[selected_unit] <= 1'b0;
                    unit_done[selected_unit] <= 1'b1;
                    current_state <= IDLE;
                end

                default: current_state <= IDLE;
            endcase

            // エラー検出時の処理
            if (|mem_error) begin
                current_state <= ERROR_HANDLE;
            end
        end
    end

    // synthesis translate_off
    // デバッグ用モニタリング
    always @(posedge clk) begin
        if (current_state == ERROR_HANDLE) begin
            $display("Memory Controller Error: Unit=%0d, Error=0x%0h", 
                    selected_unit, mem_error);
        end
    end
    // synthesis translate_on

endmodule