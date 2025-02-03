`ifndef MTX_TYPES_PKG
`define MTX_TYPES_PKG

package mtx_types;
    // 定数定義
    localparam R = 16;  // 行数
    localparam C = 16;  // 列数
    localparam V = 16;  // ベクトルサイズ
    localparam Q = 31;  // 固定小数点精度

    // 三値要素型
    typedef enum logic [1:0] {
        ZERO  = 2'b00,
        PLUS  = 2'b01,
        MINUS = 2'b10
    } val3_t;

    // 固定小数点型
    typedef logic signed [31:0] q31_t;

    // 命令型
    typedef enum logic [4:0] {
        NOP        = 5'b00000,  // 何もしない命令

        // データ移動命令
        LD_V0     = 5'b01000,  // ベクトルレジスタV0にロード
        LD_V1     = 5'b01001,  // ベクトルレジスタV1にロード
        LD_M0     = 5'b01010,  // 行列レジスタM0にロード
        
        // データ移動命令
        ST_V0     = 5'b01011,  // V0から取り出し
        ST_V1     = 5'b01100,  // V1から取り出し
        ST_M0     = 5'b01101,  // M0から取り出し

        // 演算命令
        MVMUL     = 5'b00001,  // V0 = M0 x V0
        VADD_01   = 5'b00010,  // V0 += V1
        VSUB_01   = 5'b00011,  // V0 -= V1
        
        // 初期化命令
        ZERO_V0   = 5'b01110,  // V0をゼロ初期化
        ZERO_V1   = 5'b01111,  // V1をゼロ初期化
        ZERO_M0   = 5'b10000,  // M0をゼロ初期化

        // メモリ関連命令
        PUSH_V0   = 5'b10001,  // V0を共有メモリにコピー
        POP_V1    = 5'b10010,  // 共有メモリからV1にコピー

        // 活性化関数
        VRELU     = 5'b10011,  // V0にReLU適用
        VHTANH    = 5'b10100,  // V0にHard Tanh適用
        VSQR      = 5'b10101   // V0 = V0^2
    } op_t;

    // VLIW命令構造体（4段）
    typedef struct packed {
        op_t op1;
        op_t op2;
        op_t op3;
        op_t op4;
    } vliw_inst_t;

    // 行列/ベクトル共用体型
    typedef union packed {
        struct packed {
            val3_t [R-1:0][C-1:0] data3;
        } mtx;
        
        struct packed {
            q31_t [V-1:0] vec;
        } vec;
        
        logic [511:0] raw;
    } mv_t;

    // 演算状態
    typedef struct packed {
        logic of;      // オーバーフロー
        logic uf;      // アンダーフロー
        logic zero;    // ゼロ
        logic inv;     // 不正
    } status_t;

    // 飽和演算
    function automatic q31_t sat(input logic signed [63:0] x);
        if (x > 64'sh7FFFFFFF)
            return 32'h7FFFFFFF;
        else if (x < -64'sh7FFFFFFF)
            return 32'h80000000;
        else
            return x[31:0];
    endfunction

    // 三値乗算
    function automatic q31_t mul3(
        input val3_t v, 
        input q31_t x
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
