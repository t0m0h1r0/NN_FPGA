// unit.sv
module unit
    import accel_pkg::*;
(
    // 基本インターフェース
    input  logic clk,
    input  logic rst_n,
    input  logic [1:0] unit_id,
    
    // 最適化された制御インターフェース
    input  control_packet_t control,
    output logic ready,
    output logic done,
    
    // リソース共有インターフェース
    input  logic global_resource_available,
    output logic request_global_resource,
    
    // メモリインターフェース
    output logic mem_request,
    output logic [3:0] mem_op_type,
    output logic [3:0] vec_index,
    output logic [3:0] mat_row,
    output logic [3:0] mat_col,
    output vector_data_t write_data,
    input  logic mem_grant,
    input  vector_data_t read_data,
    input  logic mem_done,
    
    // データインターフェース
    input  vector_data_t data_in,
    input  matrix_data_t matrix_in,
    output vector_data_t data_out,
    
    // キャッシュインターフェース
    output logic [3:0] cache_op,
    output logic [5:0] cache_addr,
    output vector_data_t cache_write_data,
    input  vector_data_t cache_read_data,
    input  logic cache_hit
);
    // 内部状態と制御信号
    control_signal_t decoded_control;
    logic decode_valid;
    logic [1:0] error_status;

    // キャッシュ制御用の内部信号
    logic cache_request;
    logic cache_write_enable;

    // デコーダインスタンス
    optimized_decoder u_decoder (
        .clk(clk),
        .rst_n(rst_n),
        .encoded_control({control.encoded_control, control.data_control}),
        .decoded_control(decoded_control),
        .decode_valid(decode_valid),
        .error_status(error_status)
    );

    // リソース管理用の状態
    typedef enum logic [2:0] {
        ST_IDLE,
        ST_CACHE_CHECK,
        ST_RESOURCE_REQUEST,
        ST_MEMORY_ACCESS,
        ST_COMPUTE,
        ST_WRITE_BACK
    } unit_state_t;

    unit_state_t current_state;
    
    // リソース管理と状態遷移
    always_ff @(posedge clk or negedge rst_n) begin
        if (!rst_n) begin
            reset_unit();
        end
        else begin
            // エラー処理を最優先
            if (|error_status) begin
                handle_error();
                return;
            end

            // メイン状態遷移
            case (current_state)
                ST_IDLE:        handle_idle_state();
                ST_CACHE_CHECK: handle_cache_check();
                ST_RESOURCE_REQUEST: handle_resource_request();
                ST_MEMORY_ACCESS: handle_memory_access();
                ST_COMPUTE:     handle_compute_state();
                ST_WRITE_BACK:  handle_write_back();
            endcase
        end
    end

    // リセットタスク
    task reset_unit();
        ready <= 1'b1;
        done <= 1'b0;
        mem_request <= 1'b0;
        request_global_resource <= 1'b0;
        current_state <= ST_IDLE;
        cache_request <= 1'b0;
        cache_write_enable <= 1'b0;
    endtask

    // アイドル状態のハンドリング
    task handle_idle_state();
        if (decode_valid) begin
            current_state <= ST_CACHE_CHECK;
            ready <= 1'b0;
            
            // キャッシュチェック用のアドレス設定
            cache_addr <= {unit_id, decoded_control.addr};
        end
    endtask

    // キャッシュチェック
    task handle_cache_check();
        if (cache_hit) begin
            // キャッシュヒット時の処理
            data_out <= cache_read_data;
            done <= 1'b1;
            current_state <= ST_IDLE;
            ready <= 1'b1;
        end
        else begin
            // キャッシュミス時はグローバルリソースをリクエスト
            current_state <= ST_RESOURCE_REQUEST;
            request_global_resource <= 1'b1;
        end
    endtask

    // リソースリクエスト
    task handle_resource_request();
        if (global_resource_available) begin
            current_state <= ST_MEMORY_ACCESS;
            mem_request <= 1'b1;
            
            // 操作タイプに応じた設定
            case (decoded_control.op_code)
                OP_LOAD:  set_load_operation();
                OP_STORE: set_store_operation();
                OP_COMP:  set_compute_operation();
                default:  reset_unit();
            endcase
        end
    endtask

    // メモリアクセス
    task handle_memory_access();
        if (mem_done) begin
            // キャッシュへの書き込み
            cache_request <= 1'b1;
            cache_write_enable <= 1'b1;
            cache_addr <= {unit_id, decoded_control.addr};
            cache_write_data <= read_data;
            
            // 計算が必要な場合は次の状態へ
            if (decoded_control.op_code == OP_COMP) begin
                current_state <= ST_COMPUTE;
            end
            else begin
                current_state <= ST_WRITE_BACK;
            end
        end
    endtask

    // 計算状態
    task handle_compute_state();
        // 計算ロジックを実行
        perform_computation(decoded_control.comp_type);
        
        current_state <= ST_WRITE_BACK;
    endtask

    // ライトバック
    task handle_write_back();
        done <= 1'b1;
        ready <= 1'b1;
        current_state <= ST_IDLE;
        request_global_resource <= 1'b0;
    endtask

    // エラー処理
    task handle_error();
        reset_unit();
    endtask

    // 各種オペレーション設定タスク
    task set_load_operation();
        mem_op_type <= 4'b0001;
        vec_index <= decoded_control.addr;
    endtask

    task set_store_operation();
        mem_op_type <= 4'b0010;
        vec_index <= decoded_control.addr;
        write_data <= data_in;
    endtask

    task set_compute_operation();
        mem_op_type <= 4'b0100;
    endtask

    // 計算タスク
    task perform_computation(input computation_type_t comp_type);
        case (comp_type)
            COMP_ADD:  perform_addition();
            COMP_MUL:  perform_matrix_multiplication();
            COMP_TANH: perform_tanh_activation();
            COMP_RELU: perform_relu_activation();
        endcase
    endtask

    // 既存の計算タスク（以前のコードと同様）
    task perform_addition();
        // 加算ロジック
    endtask

    task perform_matrix_multiplication();
        // 行列乗算ロジック
    endtask

    task perform_tanh_activation();
        // Tanh活性化関数ロジック
    endtask

    task perform_relu_activation();
        // ReLU活性化関数ロジック
    endtask

    // デバッグ用モニタリング
    // synthesis translate_off
    always @(posedge clk) begin
        if (current_state != ST_IDLE) begin
            $display("Unit %0d: state=%0d, op_code=%0d", 
                    unit_id, current_state, decoded_control.op_code);
        end
    end
    // synthesis translate_on
endmodule