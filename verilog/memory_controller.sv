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
    // 内部状態定義（簡素化）
    typedef enum logic [2:0] {
        ST_IDLE,
        ST_ARBITRATE,
        ST_ACCESS,
        ST_COMPLETE,
        ST_ERROR
    } ctrl_state_t;

    // 内部信号
    ctrl_state_t current_state;
    logic [1:0] selected_unit;
    logic [3:0] priority_mask;
    
    // アドレス生成
    logic [5:0] vector_addr;
    logic [7:0] matrix_addr;
    
    // アドレス生成モジュールのインスタンス化
    memory_address_generator addr_gen (
        .unit_id(selected_unit),
        .vector_index(unit_vec_index[selected_unit]),
        .matrix_row(unit_mat_row[selected_unit]),
        .matrix_col(unit_mat_col[selected_unit]),
        .vector_addr(vector_addr),
        .matrix_addr(matrix_addr)
    );

    // 優先順位に基づくユニット選択関数
    function automatic logic [1:0] select_next_unit(
        input logic [3:0] requests,
        input logic [3:0] priority_mask
    );
        logic [3:0] masked_requests = requests & priority_mask;
        
        // 優先度の高いユニットから順に選択
        unique case (1'b1)
            masked_requests[0]: return 2'd0;
            masked_requests[1]: return 2'd1;
            masked_requests[2]: return 2'd2;
            masked_requests[3]: return 2'd3;
            default: return 2'd0;
        endcase
    endfunction

    // メインステートマシン
    always_ff @(posedge clk or negedge rst_n) begin
        if (!rst_n) begin
            // リセット時の初期化
            reset_controller();
        end
        else begin
            // メモリエラー処理
            if (|mem_error) begin
                current_state <= ST_ERROR;
            end
            
            // メインステート遷移
            case (current_state)
                ST_IDLE: handle_idle_state();
                ST_ARBITRATE: handle_arbitrate_state();
                ST_ACCESS: handle_access_state();
                ST_COMPLETE: handle_complete_state();
                ST_ERROR: handle_error_state();
            endcase
        end
    end

    // 各状態のハンドリングタスク
    task reset_controller();
        current_state <= ST_IDLE;
        unit_grant <= '0;
        unit_done <= '0;
        priority_mask <= 4'b0001;  // 初期優先順位
        mem_we_a <= 1'b0;
        mem_we_b <= 1'b0;
    endtask

    task handle_idle_state();
        if (|unit_request) begin
            current_state <= ST_ARBITRATE;
            unit_done <= '0;
        end
    endtask

    task handle_arbitrate_state();
        // ユニットの選択と優先順位の更新
        selected_unit <= select_next_unit(unit_request, priority_mask);
        priority_mask <= {priority_mask[2:0], priority_mask[3]};
        current_state <= ST_ACCESS;
    endtask

    task handle_access_state();
        // メモリアドレスと操作の設定
        mem_addr_a <= vector_addr;
        mem_addr_b <= matrix_addr;
        
        // ユニット固有の操作
        case (unit_op_type[selected_unit])
            4'b0001: handle_load_operation();  // Load
            4'b0010: handle_store_operation();  // Store
            4'b0100: handle_compute_operation();  // Compute
            default: current_state <= ST_IDLE;
        endcase
    endtask

    task handle_load_operation();
        mem_we_a <= 1'b0;
        mem_we_b <= 1'b0;
        unit_grant[selected_unit] <= 1'b1;
        
        if (!mem_busy) begin
            unit_read_data[selected_unit] <= mem_rdata_a;
            current_state <= ST_COMPLETE;
        end
    endtask

    task handle_store_operation();
        mem_we_a <= 1'b1;
        mem_wdata_a <= unit_write_data[selected_unit].data[0];  // 最初の要素のみ
        unit_grant[selected_unit] <= 1'b1;
        
        if (!mem_busy) begin
            current_state <= ST_COMPLETE;
        end
    endtask

    task handle_compute_operation();
        // コンピュート操作の基本的なハンドリング
        mem_we_a <= 1'b0;
        unit_grant[selected_unit] <= 1'b1;
        
        if (!mem_busy) begin
            current_state <= ST_COMPLETE;
        end
    endtask

    task handle_complete_state();
        // アクセス完了処理
        unit_grant[selected_unit] <= 1'b0;
        unit_done[selected_unit] <= 1'b1;
        current_state <= ST_IDLE;
    endtask

    task handle_error_state();
        // エラー時のリセット処理
        reset_controller();
    endtask

    // デバッグ用モニタリング
    // synthesis translate_off
    always @(posedge clk) begin
        if (current_state == ST_ERROR) begin
            $display("Memory Controller Error: Unit=%0d, Error=0x%0h", 
                    selected_unit, mem_error);
        end
    end
    // synthesis translate_on
endmodule