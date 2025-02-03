`ifndef MTX_TYPES_PKG
`define MTX_TYPES_PKG

package mtx_types #(
    // 精度設定用パラメータ
    parameter Q = 23,     // 小数部のビット数（デフォルト23）
    parameter INT = 8,    // 整数部のビット数（デフォルト8）
    parameter TOTAL = INT + Q + 1  // 総ビット数（+1は符号ビット）
);
    // サイズ定数
    localparam R = 16;     // 行数
    localparam C = 16;     // 列数
    localparam V = 16;     // ベクトルサイズ

    // 型チェック用定数
    localparam MIN_Q = 16;      // 最小小数部ビット数
    localparam MAX_Q = 29;      // 最大小数部ビット数
    localparam MIN_INT = 2;     // 最小整数部ビット数
    localparam MAX_INT = 12;    // 最大整数部ビット数
    
    // パラメータ範囲チェック
    initial begin
        if (Q < MIN_Q || Q > MAX_Q)
            $error("小数部ビット数は%dから%dの間である必要があります", MIN_Q, MAX_Q);
        if (INT < MIN_INT || INT > MAX_INT)
            $error("整数部ビット数は%dから%dの間である必要があります", MIN_INT, MAX_INT);
        if (TOTAL != INT + Q + 1)
            $error("総ビット数は整数部+小数部+符号ビットと一致する必要があります");
    end

    // 三値要素型
    typedef enum logic [1:0] {
        ZERO  = 2'b00,
        PLUS  = 2'b01,
        MINUS = 2'b10
    } val3_t;

    // ベクトル用固定小数点型（可変精度）
    typedef logic signed [TOTAL-1:0] qformat_t;

    // ベクトル型（V個のQフォーマット）
    typedef struct packed {
        qformat_t [V-1:0] elements;
    } vec_t;

    // 行列型（R×Cの三値行列）
    typedef struct packed {
        val3_t [R-1:0][C-1:0] elements;
    } mtx_t;

    // データ共用型（ベクトルまたは行列として解釈可能）
    typedef union packed {
        mtx_t mtx;
        vec_t vec;
        logic [511:0] raw;
    } mv_t;

    // 命令型
    typedef enum logic [4:0] {
        NOP        = 5'b00000,
        LD_V0     = 5'b01000,
        LD_V1     = 5'b01001,
        LD_M0     = 5'b01010,
        ST_V0     = 5'b01011,
        ST_V1     = 5'b01100,
        ST_M0     = 5'b01101,
        MVMUL     = 5'b00001,
        VADD_01   = 5'b00010,
        VSUB_01   = 5'b00011,
        ZERO_V0   = 5'b01110,
        ZERO_V1   = 5'b01111,
        ZERO_M0   = 5'b10000,
        PUSH_V0   = 5'b10001,
        PULL_V1   = 5'b10010,
        PULL_V0   = 5'b10011,
        VRELU     = 5'b10100,
        VHTANH    = 5'b10101,
        VSQR      = 5'b10110
    } op_t;

    // VLIW命令構造体（4段）
    typedef struct packed {
        op_t op1;
        op_t op2;
        op_t op3;
        op_t op4;
    } vliw_inst_t;

    // 演算状態
    typedef struct packed {
        logic of;      // オーバーフロー
        logic uf;      // アンダーフロー
        logic zero;    // ゼロ
        logic inv;     // 不正
    } status_t;

    // 三値乗算（Qフォーマット対応）
    function automatic qformat_t mul3_qformat(
        input val3_t v, 
        input qformat_t x
    );
        unique case(v)
            ZERO:  return '0;
            PLUS:  return x;
            MINUS: return -x;
            default: return '0;
        endcase
    endfunction
endpackage

`endif