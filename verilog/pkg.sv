// pkg.sv
package accel_pkg;
    // システム定数
    localparam VECTOR_WIDTH = 32;
    localparam DATA_DEPTH   = 16;
    localparam UNIT_COUNT   = 256;
    localparam UNIT_ID_WIDTH= 8;

    // 命令エンコーディング
    typedef enum logic [1:0] {
        OP_NOP    = 2'b00,
        OP_LOAD   = 2'b01,
        OP_STORE  = 2'b10,
        OP_COMPUTE= 2'b11
    } op_type_e;

    // 計算タイプ
    typedef enum logic [1:0] {
        COMP_ADD  = 2'b00,
        COMP_MUL  = 2'b01,
        COMP_TANH = 2'b10,
        COMP_RELU = 2'b11
    } comp_type_e;

    // データ構造  
    typedef struct packed {
        logic [VECTOR_WIDTH-1:0] data [DATA_DEPTH];
    } vector_t;

    typedef struct packed {
        logic [1:0] data [DATA_DEPTH][DATA_DEPTH];
    } matrix_t;

    typedef union packed {
        vector_t vector;
        matrix_t matrix;  
    } data_t;

    // 制御パケット
    typedef struct packed {
        logic [UNIT_ID_WIDTH-1:0] unit_id;
        logic [5:0] ctrl;
        logic [7:0] config;
    } ctrl_packet_t;

    // デコード後の制御信号
    typedef struct packed {
        logic [UNIT_ID_WIDTH-1:0] unit_id;
        op_type_e op_code;
        comp_type_e comp_type;
        logic [3:0] addr;
        logic valid;
        logic [2:0] size;
    } decoded_ctrl_t;
endpackage