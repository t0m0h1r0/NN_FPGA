// pkg.sv
package accel_pkg;
    // 基本パラメータ（変更なし）
    parameter VECTOR_WIDTH = 32;
    parameter VECTOR_DEPTH = 16;
    parameter MATRIX_DEPTH = 16;
    parameter NUM_PROCESSING_UNITS = 4;

    // 状態定義の簡素化
    typedef enum logic [1:0] {
        ST_IDLE      = 2'b00,
        ST_TRANSFER  = 2'b01,
        ST_COMPUTE   = 2'b10
    } unit_state_t;

    // オペコードの定義（変更なし）
    typedef enum logic [1:0] {
        OP_NOP   = 2'b00,
        OP_LOAD  = 2'b01,
        OP_STORE = 2'b10,
        OP_COMP  = 2'b11
    } operation_code_t;

    // 計算タイプの定義（変更なし）
    typedef enum logic [1:0] {
        COMP_ADD  = 2'b00,
        COMP_MUL  = 2'b01,
        COMP_TANH = 2'b10,
        COMP_RELU = 2'b11
    } computation_type_t;

    // データ構造の定義（変更なし）
    typedef struct packed {
        logic [VECTOR_WIDTH-1:0] data [VECTOR_DEPTH];
    } vector_data_t;

    typedef struct packed {
        logic [1:0] data [MATRIX_DEPTH][MATRIX_DEPTH];
    } matrix_data_t;

    // 制御パケットの定義（最適化）
    typedef struct packed {
        logic [5:0] encoded_control;  // より明確な制御信号エンコーディング
        logic [7:0] data_control;     // データ制御の柔軟性向上
    } control_packet_t;

    // デコード後の制御信号（最適化）
    typedef struct packed {
        logic [1:0] unit_id;
        operation_code_t op_code;
        computation_type_t comp_type;
        logic [3:0] addr;
        logic valid;
        logic [2:0] size;
    } control_signal_t;

endpackage