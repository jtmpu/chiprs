; Calculates 10 * 2, by iterating 10 times with an add
main:
    ldb r1 0
    ldb r2 0
loop:
    sne r1 10
    jmp exit
    add r1 1
    add r2 2
    jmp loop
exit:
    exit
