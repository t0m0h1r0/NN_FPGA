`ifndef MTX_UNIT_TESTBENCH
`define MTX_UNIT_TESTBENCH

`include "mtx_unit.sv"
`include "shared_memory.sv"

module mtx_unit_tb;
    import mtx_types::*;

    // テスト信号
    logic clk, rst_n;
    vliw_inst_t vliw_inst;
    mv_t in, out;
    status_t st;

    // 共有メモリ信号
    mv_t global_shared_mem_in, global_shared_mem_out;
    logic [4:0] write_unit_id, read_unit_id;
    logic write_enable;
    mv_t shared_mem_read_data;

    // インスタンス化
    mtx_unit dut(
        .clk(clk),
        .rst_n(rst_n),
        .vliw_inst(vliw_inst),
        .in(in),
        .global_shared_mem_in(global_shared_mem_in),
        .global_shared_mem_out(global_shared_mem_out),
        .out(out),
        .st(st)
    );

    // 共有メモリインスタンス
    shared_memory shared_mem(
        .clk(clk),
        .rst_n(rst_n),
        .write_unit_id(write_unit_id),
        .read_unit_id(read_unit_id),
        .write_data(global_shared_mem_out),
        .write_enable(write_enable),
        .read_data(shared_mem_read_data)
    );

    // クロック生成
    always #5 clk = ~clk;

    // テストシーケンス
    initial begin
        // 初期化
        clk = 0;
        rst_n = 0;
        vliw_inst = '0;
        in = '0;
        global_shared_mem_in = '0;
        write_unit_id = '0;
        read_unit_id = '0;
        write_enable = 0;

        // リセット解除
        #10 rst_n = 1;

        // V0にデータをロード
        #20 
        vliw_inst = '{
            op1: LD_V0,
            op2: NOP,
            op3: NOP,
            op4: NOP
        };
        
        // テストデータ設定
        for (int i = 0; i < V; i++) begin
            // いくつかのテストパターン
            in.vec.vec[i] = (1 << 30) + (i << 24);
        end

        // V1にも同様のデータをロード
        #30
        vliw_inst = '{
            op1: LD_V1,
            op2: NOP,
            op3: NOP,
            op4: NOP
        };
        in = dut.V0;  // 前回のV0データを使用

        // M0に行列データをロード
        #40
        vliw_inst = '{
            op1: LD_M0,
            op2: NOP,
            op3: NOP,
            op4: NOP
        };
        
        // 三値行列データの設定
        for (int r = 0; r < R; r++) begin
            for (int c = 0; c < C; c++) begin
                // 交互に+1, 0, -1を設定
                in.mtx.data3[r][c] = 
                    (r + c) % 3 == 0 ? PLUS :
                    (r + c) % 3 == 1 ? ZERO : MINUS;
            end
        end

        // 行列-ベクトル乗算
        #50
        vliw_inst = '{
            op1: MVMUL,
            op2: NOP,
            op3: NOP,
            op4: NOP
        };

        // V0を共有メモリにPUSH
        #60
        vliw_inst = '{
            op1: PUSH_V0,
            op2: NOP,
            op3: NOP,
            op4: NOP
        };
        write_unit_id = 5'b00001;  // ユニットID 1に書き込み
        write_enable = 1;

        // V1をゼロ初期化
        #70
        vliw_inst = '{
            op1: ZERO_V1,
            op2: NOP,
            op3: NOP,
            op4: NOP
        };

        // 共有メモリからV1にPULL
        #80
        vliw_inst = '{
            op1: PULL_V1,
            op2: NOP,
            op3: NOP,
            op4: NOP
        };
        read_unit_id = 5'b00001;  // ユニットID 1から読み出し
        global_shared_mem_in = shared_mem_read_data;

        // ReLU適用
        #90
        vliw_inst = '{
            op1: VRELU,
            op2: NOP,
            op3: NOP,
            op4: NOP
        };

        // シミュレーション終了
        #100 $finish;
    end

    // オプション: デバッグ用波形出力
    initial begin
        $dumpfile("mtx_unit_tb.vcd");
        $dumpvars(0, mtx_unit_tb);
    end
endmodule

`endif