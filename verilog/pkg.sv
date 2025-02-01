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
        TRANSFER  = 2'b01,
        COMPUTE   = 2'b10
    } unit_state_t;

    // 最適化されたオペコード
    typedef enum logic [1:0] {
        OP_NOP   = 2'b00,  // 無操作
        OP_LOAD  = 2'b01,  // データロード
        OP_STORE = 2'b10,  // データストア
        OP_COMP  = 2'b11   // 計算操作
    } operation_code_t;

    // 計算タイプの定義
    typedef enum logic [1:0] {
        COMP_ADD  = 2'b00,  // ベクトル加算
        COMP_MUL  = 2'b01,  // 行列ベクトル乗算
        COMP_TANH = 2'b10,  // tanh活性化
        COMP_RELU = 2'b11   // ReLU活性化
    } computation_type_t;

    // データ構造の定義
    typedef struct packed {
        logic [VECTOR_WIDTH-1:0] data [VECTOR_DEPTH];
    } vector_data_t;

    typedef struct packed {
        logic [1:0] data [MATRIX_DEPTH][MATRIX_DEPTH];
    } matrix_data_t;

    // 最適化された制御パケット定義
    typedef struct packed {
        logic [5:0] encoded_control;  // [5:4]:unit_id, [3:2]:op_code, [1:0]:comp_type
        logic [7:0] data_control;     // [7:4]:addr, [3]:valid, [2:0]:size
    } control_packet_t;

    // デコード後の制御信号
    typedef struct packed {
        logic [1:0] unit_id;
        operation_code_t op_code;
        computation_type_t comp_type;
        logic [3:0] addr;
        logic valid;
        logic [2:0] size;
    } control_signal_t;

endpackage