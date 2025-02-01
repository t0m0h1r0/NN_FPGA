// pkg.sv
package accel_pkg;
    // 基本パラメータ
    parameter VECTOR_WIDTH = 32;
    parameter VECTOR_DEPTH = 16;
    parameter MATRIX_DEPTH = 16;
    parameter NUM_PROCESSING_UNITS = 4;

    // 簡略化された状態定義
    typedef enum logic [1:0] {
        IDLE      = 2'b00,
        FETCH     = 2'b01,
        COMPUTE   = 2'b10,
        WRITEBACK = 2'b11
    } unit_state_t;

    // 最適化されたオペコード
    typedef enum logic [2:0] {
        OP_NOP   = 3'b000,  // 無操作
        OP_LOAD  = 3'b001,  // データロード
        OP_STORE = 3'b010,  // データストア
        OP_COMP  = 3'b011,  // 計算操作
        OP_SYNC  = 3'b100   // 同期操作
    } operation_code_t;

    // 計算タイプの定義
    typedef enum logic [2:0] {
        COMP_ADD    = 3'b000,  // ベクトル加算
        COMP_MUL    = 3'b001,  // 行列ベクトル乗算
        COMP_TANH   = 3'b010,  // tanh活性化
        COMP_RELU   = 3'b011   // ReLU活性化
    } computation_type_t;

    // データ構造の定義
    typedef struct packed {
        logic [VECTOR_WIDTH-1:0] data [VECTOR_DEPTH];
    } vector_data_t;

    typedef struct packed {
        logic [1:0] data [MATRIX_DEPTH][MATRIX_DEPTH];
    } matrix_data_t;

    // 制御パケット定義
    typedef struct packed {
        logic [3:0] unit_id;
        operation_code_t op_code;
        computation_type_t comp_type;
        logic [3:0] reserved;
    } control_packet_t;
endpackage