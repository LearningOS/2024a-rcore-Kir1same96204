## 实现功能总结
1. 为`TaskControlBlock`新增两个字段
    - `start_time` ：记录此task开始执行时间，单位ms。初始化为0。在调用`run_next_task`时，，首先检查下一个task的此字段是否为0，若为0则说明是第一次执行，赋值为当前时间。
    - `syscall_times`：记录系统调用次数的桶数组。采用桶计数会占用大量内存，但实现比较简单，而且目前task数较小。还可以考虑使用哈希表维护。

2. 为`task`新增维护和获取上述两个字段的方法
3. 实现`sys_task_info`系统调用

## 简答作业
1.  - `ch2b_bad_address.rs`: [kernel] PageFault in application, bad addr = 0x0, bad instruction = 0x804003a4, kernel killed it.
    - `ch2b_bad_instructions.rs`: [kernel] IllegalInstruction in application, kernel killed it.
    - `ch2b_bad_register.rs.rs`: [kernel] IllegalInstruction in application, kernel killed it.
    
    RustSBI版本 0.3.0-alpha.2

2. 
    1. a0指向分配 Trap 上下文之后的内核栈栈顶。`__restore` 的使用情景：一是开始运行第一个应用前，需要构造好应用程序开始执行所需的 Trap 上下文，然后调用`__restore`从刚构造的 Trap 上下文中，恢复应用程序执行的部分寄存器；二是处理完trap后，需要调用`__restore`恢复被打断的应用程序的Trap 上下文。
    2. 特殊处理了`sstatus`,`sepc`和`sscratch`。 `sstatus`。`sstatus`标识trap前的特权级，若要进入的是用户态，那么应被恢复为相应的值，`sepc`记录 Trap 发生之前执行的最后一条指令的地址，需要它来得出返回用户态后执行的第一条指令的地址。`sscratch`存的是用户栈的地址。需要它来换栈。
    3. `x2`：x2即sp寄存器，需要基于它来找到每个寄存器应该被保存到的正确的位置。
    `x4`：一般不会使用
    4. sp指向用户栈，sscratch指向内核栈
    5. `sret`。该指令执行后CPU回到 U 特权级。
    6. sp指向内核栈，sscratch指向用户栈
    7. `ecall`

## 荣誉准则

1. 在完成本次实验的过程（含此前学习的过程）中，我曾分别与 以下各位 就（与本次实验相关的）以下方面做过交流，还在代码中对应的位置以注释形式记录了具体的交流对象及内容：

    无

2. 此外，我也参考了 以下资料 ，还在代码中对应的位置以注释形式记录了具体的参考来源及内容：

    无

3. 我独立完成了本次实验除以上方面之外的所有工作，包括代码与文档。 我清楚地知道，从以上方面获得的信息在一定程度上降低了实验难度，可能会影响起评分。

4. 我从未使用过他人的代码，不管是原封不动地复制，还是经过了某些等价转换。 我未曾也不会向他人（含此后各届同学）复制或公开我的实验代码，我有义务妥善保管好它们。 我提交至本实验的评测系统的代码，均无意于破坏或妨碍任何计算机系统的正常运转。 我清楚地知道，以上情况均为本课程纪律所禁止，若违反，对应的实验成绩将按“-100”分计。