// memory_controller.sv
module memory_controller
    import accel_pkg::*;
(
    input  logic clk,
    input  logic rst_n,

    // ユニットインターフェース
    input  logic [UNIT_COUNT-1:0] unit_request,
    input  logic [UNIT_COUNT-1:0] [3:0] unit_op_type,
    input  logic [UNIT_COUNT-1:0] [3:0] unit_vec_index,
    input  logic [UNIT_COUNT-1:0] [3:0] unit_mat_row,
    input  logic [UNIT_COUNT-1:0] [3:0] unit_mat_col,
    input  vector_t unit_write_data [UNIT_COUNT],
    output logic [UNIT_COUNT-1:0] unit_grant,
    output vector_t unit_read_data [UNIT_COUNT],
    output logic [UNIT_COUNT-1:0] unit_done,

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
    // メモリアクセス状態
    typedef enum logic [2:0] {
        ST_IDLE,
        ST_ARBITRATE,
        ST_ACCESS,
        ST_COMPLETE,
        ST_ERROR
    } mem_ctrl_state_e;

    // 内部状態
    mem_ctrl_state_e current_state;
    logic [1:0] selected_unit;
    logic [3:0] priority_mask;
    
    // アドレス生成
    logic [5:0] vector_addr;
    logic [7:0] matrix_addr;

    // アドレス生成
    always_comb begin
        vector_addr = {selected_unit, unit_vec_index[selected_unit]};
        matrix_addr = {unit_mat_row[selected_unit], unit_mat_col[selected_unit]};
    end

    // メインステートマシン
    always_ff @(posedge clk or negedge rst_n) begin
        if (!rst_n) begin
            reset_controller();
        end
        else begin
            // メモリエラー処理
            if (|mem_error) begin
                current_state <= ST_ERROR;
            end
            
            // ステート遷移
            case (current_state)
                ST_IDLE:       handle_idle_state();
                ST_ARBITRATE:  handle_arbitrate_state();
                ST_ACCESS:     handle_access_state();
                ST_COMPLETE:   handle_complete_state();
                ST_ERROR:      handle_error_state();
            endcase
        end
    end

    // コントローラリセット
    task reset_controller();
        current_state <= ST_IDLE;
        unit_grant <= '0;
        unit_done <= '0;
        priority_mask <= 4'b0001;  // 初期優先順位
        mem_we_a <= 1'b0;
        mem_we_b <= 1'b0;
    endtask

    // アイドル状態ハンドリング
    task handle_idle_state();
        if (|unit_request) begin
            current_state <= ST_ARBITRATE;
            unit_done <= '0;
        end
    endtask

    // ユニット選択
    function automatic logic [1:0] select_next_unit(
        input logic [UNIT_COUNT-1:0] requests,
        input logic [3:0] priority_mask
    );
        logic [UNIT_COUNT-1:0] masked_requests = requests & priority_mask;
        
        // 優先度の高いユニットから順に選択
        unique case (1'b1)
            masked_requests[0]: return 2'd0;
            masked_requests[1]: return 2'd1;
            masked_requests[2]: return 2'd2;
            masked_requests[3]: return 2'd3;
            default: return 2'd0;
        endcase
    endfunction

    // アービトレーション状態
    task handle_arbitrate_state();
        // ユニット選択と優先順位更新
        selected_unit <= select_next_unit(unit_request, priority_mask);
        priority_mask <= {priority_mask[2:0], priority_mask[3]};
        current_state <= ST_ACCESS;
    endtask

    // メモリアクセス状態
    task handle_access_state();
        // メモリアドレスと操作の設定
        mem_addr_a <= vector_addr;
        mem_addr_b <= matrix_addr;
        
        // 操作タイプに応じた処理
        unique case (unit_op_type[selected_unit])
            4'b0001: handle_load_operation();  // Load
            4'b0010: handle_store_operation();  // Store
            4'b0100: handle_compute_operation();  // Compute
            default: current_state <= ST_IDLE;
        endcase
    endtask

    // ロード操作ハンドリング
    task handle_load_operation();
        mem_we_a <= 1'b0;
        mem_we_b <= 1'b0;
        unit_grant[selected_unit] <= 1'b1;
        
        if (!mem_busy) begin
            unit_read_data[selected_unit] <= mem_rdata_a;
            current_state <= ST_COMPLETE;
        end
    endtask

    // ストア操作ハンドリング
    task handle_store_operation();
        mem_we_a <= 1'b1;
        mem_wdata_a <= unit_write_data[selected_unit].data[0];
        unit_grant[selected_unit] <= 1'b1;
        
        if (!mem_busy) begin
            current_state <= ST_COMPLETE;
        end
    endtask

    // 計算操作ハンドリング
    task handle_compute_operation();
        mem_we_a <= 1'b0;
        unit_grant[selected_unit] <= 1'b1;
        
        if (!mem_busy) begin
            current_state <= ST_COMPLETE;
        end
    endtask

    // 完了状態ハンドリング
    task handle_complete_state();
        unit_grant[selected_unit] <= 1'b0;
        unit_done[selected_unit] <= 1'b1;
        current_state <= ST_IDLE;
    endtask

    // エラー状態ハンドリング
    task handle_error_state();
        // エラー時のリセット
        reset_controller();
    endtask

    // デバッグ用モニタリング
    // synthesis translate_off
    always @(posedge clk) begin
        if (current_state == ST_ERROR) begin
            $display("メモリコントローラエラー: Unit=%0d, Error=0x%0h", 
                    selected_unit, mem_error);
        end
    end
    // synthesis translate_on
endmodule