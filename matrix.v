// ベクトル演算ユニット
module processing_unit #(
    parameter VECTOR_WIDTH = 32,
    parameter VECTOR_DEPTH = 16,
    parameter MATRIX_DEPTH = 16
)(
    // クロックとリセット
    input wire clk,
    input wire rst_n,

    // 命令と制御
    input wire [3:0] opcode,
    output logic operation_complete,

    // データ入出力
    input wire [VECTOR_WIDTH*VECTOR_DEPTH-1:0] data_in,
    output logic [VECTOR_WIDTH*VECTOR_DEPTH-1:0] data_out,

    // 外部ユニット用
    input wire [7:0] source_unit_id  // コピー元/加算元ユニットID
);
    // オペコード定義
    typedef enum logic [3:0] {
        OP_NOP   = 4'b0000,
        OP_LD    = 4'b0001,
        OP_ST    = 4'b0010,
        OP_STM   = 4'b0011,
        OP_CLR   = 4'b0100,
        OP_CLRM  = 4'b0101,
        OP_ADD   = 4'b0110,
        OP_SUB   = 4'b0111,
        OP_MUL   = 4'b1000,
        OP_TANH  = 4'b1001,
        OP_RELU  = 4'b1010,
        OP_COPY  = 4'b1011,  // 外部ユニットからのコピー
        OP_VMADD = 4'b1100   // 外部ベクトルメモリ加算
    } opcode_t;

    // メモリ定義
    logic [VECTOR_WIDTH-1:0] vector_mem [VECTOR_DEPTH];
    logic [1:0] matrix_mem [MATRIX_DEPTH][MATRIX_DEPTH];

    // ベクトル演算サブモジュール
    function automatic logic [VECTOR_WIDTH-1:0] apply_activation (
        input logic [VECTOR_WIDTH-1:0] input_value,
        input opcode_t op_type
    );
        logic [VECTOR_WIDTH-1:0] result;
        
        // 閾値の定義
        localparam logic [VECTOR_WIDTH-1:0] TANH_THRESHOLD = 32'h40000000;  // 1.0
        localparam logic [VECTOR_WIDTH-1:0] RELU_THRESHOLD = 32'h00000000;  // 0.0
        
        case (op_type)
            OP_TANH: begin
                // TANH: [-1.0, 1.0]にクランプ
                if (input_value > TANH_THRESHOLD)
                    result = TANH_THRESHOLD;
                else if (input_value < -TANH_THRESHOLD)
                    result = -TANH_THRESHOLD;
                else
                    result = input_value;
            end
            
            OP_RELU: begin
                // RELU: 負の値を0にクランプ
                result = input_value[VECTOR_WIDTH-1] ? 
                    {VECTOR_WIDTH{1'b0}} : input_value;
            end
            
            default: result = input_value;
        endcase
        
        return result;
    endfunction

    // 行列乗算のコア関数
    function automatic logic [VECTOR_WIDTH-1:0] matrix_multiply_row(
        input logic [1:0] matrix_row [MATRIX_DEPTH],
        input logic [VECTOR_WIDTH-1:0] input_vector [VECTOR_DEPTH]
    );
        logic signed [2*VECTOR_WIDTH-1:0] temp_result = '0;
        
        for (int k = 0; k < MATRIX_DEPTH; k++) begin
            // ビット論理演算による効率的なスケール
            logic signed [VECTOR_WIDTH-1:0] scaled_vec = 
                (matrix_row[k][1] ? -input_vector[k] : input_vector[k]) & 
                {VECTOR_WIDTH{matrix_row[k][0]}};
            
            temp_result += {{VECTOR_WIDTH{scaled_vec[VECTOR_WIDTH-1]}}, scaled_vec};
        end
        
        // 飽和型飽和演算
        return (temp_result[2*VECTOR_WIDTH-1:VECTOR_WIDTH-1] > 0) ? 
               {1'b0, {VECTOR_WIDTH-1{1'b1}}} :  // 正のオーバーフロー
               (temp_result[2*VECTOR_WIDTH-1:VECTOR_WIDTH-1] < -1) ? 
               {1'b1, {VECTOR_WIDTH-1{1'b0}}} :  // 負のオーバーフロー
               temp_result[VECTOR_WIDTH-1:0];    // 通常の値
    endfunction

    // メイン処理
    always_ff @(posedge clk or negedge rst_n) begin
        if (!rst_n) begin
            // 初期化
            vector_mem <= '{default: '0};
            matrix_mem <= '{default: '0};
            data_out <= '0;
            operation_complete <= 1'b0;
        end
        else begin
            // デフォルトで完了フラグをリセット
            operation_complete <= 1'b0;

            // 通常の命令処理
            unique case (opcode_t'(opcode))
                OP_COPY, OP_VMADD: begin
                    // 現時点では何もしない（外部同期の責任）
                    operation_complete <= 1'b1;
                end

                OP_MUL: begin
                    for (int j = 0; j < VECTOR_DEPTH; j++) begin
                        vector_mem[j] <= matrix_multiply_row(
                            matrix_mem[j], 
                            vector_mem
                        );
                    end
                    operation_complete <= 1'b1;
                end

                OP_ADD, OP_SUB: begin
                    logic [VECTOR_WIDTH*VECTOR_DEPTH-1:0] input_vector;
                    
                    input_vector = (opcode == OP_SUB) ? 
                        ~data_in + 1'b1 : 
                        data_in;
                    
                    for (int j = 0; j < VECTOR_DEPTH; j++) begin
                        vector_mem[j] += input_vector[j*VECTOR_WIDTH +: VECTOR_WIDTH];
                    end
                    operation_complete <= 1'b1;
                end

                OP_TANH, OP_RELU: begin
                    for (int j = 0; j < VECTOR_DEPTH; j++) begin
                        vector_mem[j] <= apply_activation(
                            vector_mem[j], 
                            opcode_t'(opcode)
                        );
                    end
                    operation_complete <= 1'b1;
                end

                OP_LD: begin
                    data_out <= vector_mem;
                    operation_complete <= 1'b1;
                end

                OP_ST: begin
                    vector_mem <= data_in;
                    operation_complete <= 1'b1;
                end

                OP_STM: begin
                    matrix_mem <= data_in;
                    operation_complete <= 1'b1;
                end

                OP_CLR: begin
                    vector_mem <= '{default: '0};
                    operation_complete <= 1'b1;
                end

                OP_CLRM: begin
                    matrix_mem <= '{default: '0};
                    operation_complete <= 1'b1;
                end

                default: operation_complete <= 1'b0;
            endcase
        end
    end
endmodule

// マルチユニットコントローラ
module multi_unit_controller #(
    parameter NUM_UNITS = 4,
    parameter VECTOR_WIDTH = 32,
    parameter VECTOR_DEPTH = 16,
    parameter MATRIX_DEPTH = 16
)(
    // グローバル制御
    input wire clk,
    input wire rst_n,

    // ユニット間通信インターフェース
    input wire [NUM_UNITS-1:0] unit_operation_complete,
    output logic [NUM_UNITS-1:0] unit_operation_start,
    output logic [3:0] unit_opcode [NUM_UNITS],
    output logic [VECTOR_WIDTH*VECTOR_DEPTH-1:0] unit_data_in [NUM_UNITS],
    input wire [VECTOR_WIDTH*VECTOR_DEPTH-1:0] unit_data_out [NUM_UNITS]
);
    // 命令定数（processing_unitと同期）
    localparam [3:0]
        OP_COPY  = 4'b1011,
        OP_VMADD = 4'b1100;

    // 通信状態の列挙型
    typedef enum logic [2:0] {
        IDLE = 3'b000,
        PREPARE_SOURCE = 3'b001,
        WAIT_SOURCE = 3'b010,
        PREPARE_DEST = 3'b011,
        EXECUTE_COPY = 3'b100,
        EXECUTE_VMADD = 3'b101
    } system_state_t;

    // システム全体の状態管理
    system_state_t current_state;
    
    // 通信管理用レジスタ
    logic [7:0] source_unit, dest_unit;
    logic [VECTOR_WIDTH*VECTOR_DEPTH-1:0] transfer_data;

    // 状態遷移ロジック
    always_ff @(posedge clk or negedge rst_n) begin
        if (!rst_n) begin
            // リセット時の初期化
            current_state <= IDLE;
            unit_operation_start <= '0;
            source_unit <= '0;
            dest_unit <= '0;
        end
        else begin
            // デフォルト値
            unit_operation_start <= '0;

            // 状態遷移
            case (current_state)
                IDLE: begin
                    // デモ用のハードコードされた転送シーケンス
                    // 例: ユニット0からユニット1へのコピー
                    source_unit <= 8'd0;
                    dest_unit <= 8'd1;
                    current_state <= PREPARE_SOURCE;
                end

                PREPARE_SOURCE: begin
                    // ソースユニットにデータ読み出し命令を発行
                    unit_opcode[source_unit] <= 4'b0001; // OP_LD
                    unit_operation_start[source_unit] <= 1'b1;
                    current_state <= WAIT_SOURCE;
                end

                WAIT_SOURCE: begin
                    unit_operation_start[source_unit] <= 1'b0;
                    
                    if (unit_operation_complete[source_unit]) begin
                        // ソースユニットからのデータを保存
                        transfer_data <= unit_data_out[source_unit];
                        current_state <= PREPARE_DEST;
                    end
                end

                PREPARE_DEST: begin
                    // 転送先ユニットに命令を発行
                    unit_opcode[dest_unit] <= OP_COPY;
                    unit_data_in[dest_unit] <= transfer_data;
                    unit_operation_start[dest_unit] <= 1'b1;
                    current_state <= EXECUTE_COPY;
                end

                EXECUTE_COPY: begin
                    unit_operation_start[dest_unit] <= 1'b0;
                    
                    if (unit_operation_complete[dest_unit]) begin
                        // コピー完了
                        current_state <= IDLE;
                    end
                end

                default: current_state <= IDLE;
            endcase
        end
    end
endmodule

// トップレベルモジュール
module neural_network_accelerator #(
    parameter NUM_UNITS = 4,
    parameter VECTOR_WIDTH = 32,
    parameter VECTOR_DEPTH = 16,
    parameter MATRIX_DEPTH = 16
)(
    input wire clk,
    input wire rst_n
);
    // ユニット間接続用ワイヤ
    logic [NUM_UNITS-1:0] unit_operation_complete;
    logic [NUM_UNITS-1:0] unit_operation_start;
    logic [3:0] unit_opcode [NUM_UNITS];
    logic [VECTOR_WIDTH*VECTOR_DEPTH-1:0] unit_data_in [NUM_UNITS];
    logic [VECTOR_WIDTH*VECTOR_DEPTH-1:0] unit_data_out [NUM_UNITS];

    // ユニットのインスタンス化
    genvar i;
    generate
        for (i = 0; i < NUM_UNITS; i++) begin : unit_gen
            processing_unit #(
                .VECTOR_WIDTH(VECTOR_WIDTH),
                .VECTOR_DEPTH(VECTOR_DEPTH),
                .MATRIX_DEPTH(MATRIX_DEPTH)
            ) proc_unit (
                .clk(clk),
                .rst_n(rst_n),
                .opcode(unit_opcode[i]),
                .operation_complete(unit_operation_complete[i]),
                .data_in(unit_data_in[i]),
                .data_out(unit_data_out[i]),
                .source_unit_id(8'(i))  // 各ユニットに固有のID
            );
        end
    endgenerate

    // マルチユニットコントローラのインスタンス化
    multi_unit_controller #(
        .NUM_UNITS(NUM_UNITS),
        .VECTOR_WIDTH(VECTOR_WIDTH),
        .VECTOR_DEPTH(VECTOR_DEPTH),
        .MATRIX_DEPTH(MATRIX_DEPTH)
    ) controller (
        .clk(clk),
        .rst_n(rst_n),
        .unit_operation_complete(unit_operation_complete),
        .unit_operation_start(unit_operation_start),
        .unit_opcode(unit_opcode),
        .unit_data_in(unit_data_in),
        .unit_data_out(unit_data_out)
    );
endmodule