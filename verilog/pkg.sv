// pkg.sv
package accel_pkg;
    // システム定数（ビット幅拡張）
    localparam VECTOR_WIDTH  = 32;
    localparam VECTOR_DEPTH  = 16;
    localparam MATRIX_DEPTH  = 16;
    localparam UNIT_COUNT    = 256;  // 最大256ユニットに拡張
    localparam UNIT_ID_WIDTH = 8;    // 256ユニットに対応するID幅

    // 命令エンコーディング（拡張）
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
        logic [VECTOR_WIDTH-1:0] data [VECTOR_DEPTH];
    } vector_t;

    typedef struct packed {
        logic [1:0] data [MATRIX_DEPTH][MATRIX_DEPTH];
    } matrix_t;

    // 制御パケット（拡張されたユニットID）
    typedef struct packed {
        logic [UNIT_ID_WIDTH-1:0] unit_id;  // 8ビットのユニットID
        logic [5:0] ctrl;    // 制御信号
        logic [7:0] config;  // 構成情報
    } ctrl_packet_t;

    // デコード後の制御信号（拡張）
    typedef struct packed {
        logic [UNIT_ID_WIDTH-1:0] unit_id;  // 8ビットのユニットID
        op_type_e op_code;
        comp_type_e comp_type;
        logic [3:0] addr;
        logic valid;
        logic [2:0] size;
    } decoded_ctrl_t;
endpackage