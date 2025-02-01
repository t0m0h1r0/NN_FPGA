// unit.sv
module unit
    import accel_pkg::*;
(
    input  logic clk,
    input  logic rst_n,
    input  logic [1:0] unit_id,
    
    // 制御インターフェース
    input  control_packet_t control,
    output logic ready,
    output logic done,
    
    // パイプライン制御
    output logic pipeline_stall,
    input  logic pipeline_flush,
    
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
    output vector_data_t data_out
);
    // パイプラインステージ定義
    typedef enum logic [2:0] {
        FETCH,     // 命令フェッチ
        DECODE,    // デコード
        EXECUTE,   // 実行
        MEMORY,    // メモリアクセス
        WRITEBACK  // 結果書き戻し
    } pipeline_stage_t;

    // パイプラインレジスタ
    struct packed {
        control_signal_t control;
        vector_data_t data;
        matrix_data_t matrix;
        logic valid;
    } pipeline_regs [5];

    // パイプライン制御信号
    pipeline_stage_t current_stage;
    logic [2:0] stage_counter;
    
    // デコーダ
    control_signal_t decoded_control;
    logic decode_valid;
    logic [1:0] error_status;

    // デコーダインスタンス
    optimized_decoder u_decoder (
        .clk(clk),
        .rst_n(rst_n),
        .encoded_control({control.encoded_control, control.data_control}),
        .decoded_control(decoded_control),
        .decode_valid(decode_valid),
        .error_status(error_status)
    );

    // パイプライン制御
    always_ff @(posedge clk or negedge rst_n) begin
        if (!rst_n) begin
            reset_pipeline();
        end
        else if (pipeline_flush) begin
            flush_pipeline();
        end
        else begin
            // パイプラインステージの進行
            advance_pipeline_stages();
        end
    end

    // パイプラインリセットタスク
    task reset_pipeline();
        current_stage <= FETCH;
        stage_counter <= '0;
        ready <= 1'b1;
        done <= 1'b0;
        mem_request <= 1'b0;
        pipeline_stall <= 1'b0;
        
        // パイプラインレジスタのクリア
        for (int i = 0; i < 5; i++) begin
            pipeline_regs[i].valid <= 1'b0;
        end
    endtask

    // パイプラインフラッシュタスク
    task flush_pipeline();
        // 全パイプラインステージを無効化
        for (int i = 0; i < 5; i++) begin
            pipeline_regs[i].valid <= 1'b0;
        end
        current_stage <= FETCH;
        stage_counter <= '0;
        ready <= 1'b1;
        done <= 1'b0;
    endtask

    // パイプラインステージ進行タスク
    task advance_pipeline_stages();
        // 各ステージの処理
        case (current_stage)
            FETCH: handle_fetch_stage();
            DECODE: handle_decode_stage();
            EXECUTE: handle_execute_stage();
            MEMORY: handle_memory_stage();
            WRITEBACK: handle_writeback_stage();
        endcase

        // ステージカウンタと現在のステージの更新
        stage_counter <= stage_counter + 1;
        if (stage_counter == 4) begin
            current_stage <= FETCH;
            stage_counter <= '0;
        end
        else begin
            current_stage <= pipeline_stage_t'(current_stage + 1);
        end
    endtask

    // フェッチステージ
    task handle_fetch_stage();
        // 新しい命令のフェッチ
        if (decode_valid) begin
            pipeline_regs[FETCH].control <= decoded_control;
            pipeline_regs[FETCH].valid <= 1'b1;
            ready <= 1'b0;
        end
    endtask

    // デコードステージ
    task handle_decode_stage();
        if (pipeline_regs[FETCH].valid) begin
            // デコード済み命令の準備
            pipeline_regs[DECODE] <= pipeline_regs[FETCH];
        end
    endtask

    // 実行ステージ
    task handle_execute_stage();
        if (pipeline_regs[DECODE].valid) begin
            // 命令タイプに応じた実行
            case (pipeline_regs[DECODE].control.op_code)
                OP_LOAD:  execute_load();
                OP_STORE: execute_store();
                OP_COMP:  execute_compute();
                default:  ; // NOP
            endcase
        end
    endtask

    // メモリアクセスステージ
    task handle_memory_stage();
        if (pipeline_regs[EXECUTE].valid) begin
            // メモリアクセス
            mem_request <= 1'b1;
            case (pipeline_regs[EXECUTE].control.op_code)
                OP_LOAD: begin
                    mem_op_type <= 4'b0001;
                    vec_index <= pipeline_regs[EXECUTE].control.addr;
                end
                OP_STORE: begin
                    mem_op_type <= 4'b0010;
                    write_data <= data_in;
                end
                OP_COMP: begin
                    mem_op_type <= 4'b0100;
                end
            endcase
        end
    endtask

    // ライトバックステージ
    task handle_writeback_stage();
        if (pipeline_regs[MEMORY].valid) begin
            // 結果の書き戻し
            if (mem_done) begin
                data_out <= read_data;
                done <= 1'b1;
                ready <= 1'b1;
            end
        end
    endtask

    // 各種実行タスク
    task execute_load();
        // ロード命令の実行準備
        pipeline_regs[EXECUTE].data <= read_data;
    endtask

    task execute_store();
        // ストア命令の実行準備
        write_data <= data_in;
    endtask

    task execute_compute();
        // 計算命令の実行
        case (pipeline_regs[EXECUTE].control.comp_type)
            COMP_ADD:  perform_addition();
            COMP_MUL:  perform_matrix_multiplication();
            COMP_TANH: perform_tanh_activation();
            COMP_RELU: perform_relu_activation();
        endcase
    endtask

    // 計算タスク（以前と同様の実装）
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
        $display("Unit %0d: Stage=%0d, OpCode=%0d, Valid=%0b", 
                unit_id, current_stage, 
                pipeline_regs[current_stage].control.op_code,
                pipeline_regs[current_stage].valid);
    end
    // synthesis translate_on
endmodule