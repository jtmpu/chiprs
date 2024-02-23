    ldb r1 0
    ldb r2 0
    ldb r3 0
    draw r2 r3 5
    ldr r5 r1 ; save old char to remove
main:
    ldb r2 60
    delay r2
    call wait
    input r1
    call clear
    ldf r1
    ldb r2 0
    ldb r3 0
    draw r2 r3 5
    ldr r5 r1 ; save old char addr
    jmp main

clear:
    ldf r4
    ldb r2 0
    ldb r3 0
    draw r2 r3 5
    ret

wait:
    ldd r9
    sne r9 0
    ret
    jmp wait

