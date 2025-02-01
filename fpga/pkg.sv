// pkg.sv
package accel_pkg;
    // システム定数（既存）
    localparam VECTOR_WIDTH = 32;
    localparam DATA_DEPTH   = 16;
    localparam UNIT_COUNT   = 256;
    localparam UNIT_ID_WIDTH= 8;

    // 命令エンコーディング（更新）
    typedef enum logic [2:0] {
        OP_NOP     = 3'b000,
        OP_LOAD    = 3'b001,
        OP_STORE   = 3'b010,
        OP_COMPUTE = 3'b011,
        OP_COPY    = 3'b100,    // 追加：ベクトルコピー
        OP_ADD_VEC = 3'b101     // 追加：ベクトル加算
    } op_type_e;

    // 計算タイプ（既存）
    typedef enum logic [1:0] {
        COMP_ADD  = 2'b00,
        COMP_MUL  = 2'b01,
        COMP_TANH = 2'b10,
        COMP_RELU = 2'b11
    } comp_type_e;

    // データ構造（既存）  
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

    // 制御パケット（更新）
    typedef struct packed {
        logic [UNIT_ID_WIDTH-1:0] unit_id;    // ターゲットユニットID
        logic [UNIT_ID_WIDTH-1:0] src_unit_id; // 追加：ソースユニットID
        logic [5:0] ctrl;                      // 制御信号
        logic [7:0] config;                    // 設定
    } ctrl_packet_t;

    // デコード後の制御信号（更新）
    typedef struct packed {
        logic [UNIT_ID_WIDTH-1:0] unit_id;
        logic [UNIT_ID_WIDTH-1:0] src_unit_id; // 追加：ソースユニットID
        op_type_e op_code;
        comp_type_e comp_type;
        logic [3:0] addr;
        logic valid;
        logic [2:0] size;
    } decoded_ctrl_t;
endpackage