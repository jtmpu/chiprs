    mov r5 99
main:
    mov r9 30
    call wait
    delay r2
    input r1
    call clear
    ldf r1
    mov r2 0
    mov r3 0
    draw r2 r3 5
    mov r4 r1 ; save old char addr
    jmp main

clear:
    sne r4 99
    ret ; no previous char
    ldf r4
    mov r2 0
    mov r3 0
    draw r2 r3 5
    ret

wait:
    ldd r9
    sne r9 0
    jmp wait
    ret

