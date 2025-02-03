`ifndef MULTI_UNIT_PROCESSOR
`define MULTI_UNIT_PROCESSOR

`include "mtx_unit.sv"
`include "shared_memory.sv"

module multi_unit_processor(
    input logic clk,
    input logic rst_n
);
    import mtx_types::*;

    // 共有メモリ信号
    logic [4:0] write_unit_id, read_unit_id;
    logic write_enable;
    mv_t global_shared_mem_out, global_shared_mem_in;

    // 共有メモリ
    shared_memory shared_mem(
        .clk(clk),
        .rst_n(rst_n),
        .write_unit_id(write_unit_id),
        .read_unit_id(read_unit_id),
        .write_data(global_shared_mem_out),
        .write_enable(write_enable),
        .read_data(global_shared_mem_in)
    );

    // ユニット配列
    mtx_unit units[32](
        .clk(clk),
        .rst_n(rst_n),
        .global_shared_mem_in(global_shared_mem_in),
        .global_shared_mem_out(global_shared_mem_out)
        // その他の接続は省略
    );
endmodule

`endif
