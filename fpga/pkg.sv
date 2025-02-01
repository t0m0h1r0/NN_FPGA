// pkg.sv
package accel_pkg;
    // システム定数
    localparam VECTOR_WIDTH = 32;  // Q1.31形式
    localparam DATA_DEPTH   = 16;
    localparam UNIT_COUNT   = 256;
    localparam UNIT_ID_WIDTH= 8;

    // Q1.31 固定小数点形式の定義
    typedef struct packed {
        logic sign;           // 符号ビット (1ビット)
        logic [30:0] value;   // 小数部 (31ビット)
    } q1_31_t;

    // 命令エンコーディング
    typedef enum logic [2:0] {
        OP_NOP     = 3'b000,
        OP_LOAD    = 3'b001,
        OP_STORE   = 3'b010,
        OP_COMPUTE = 3'b011,
        OP_COPY    = 3'b100,
        OP_ADD_VEC = 3'b101
    } op_type_e;

    // 計算タイプ
    typedef enum logic [1:0] {
        COMP_ADD,
        COMP_MUL,
        COMP_TANH,
        COMP_RELU
    } comp_type_e;

    // デコード後の制御信号
    typedef struct packed {
        logic [UNIT_ID_WIDTH-1:0] unit_id;
        logic [UNIT_ID_WIDTH-1:0] src_unit_id;
        op_type_e op_code;
        comp_type_e comp_type;
        logic [3:0] addr;
        logic valid;
        logic [2:0] size;
    } decoded_ctrl_t;
endpackage