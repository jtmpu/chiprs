main:
    call wait
    mov r1 1
    ldf r1
    mov r1 0
    mov r2 0
    draw r1 r2 5
    call wait
    mov r1 2
    ldf r1
    mov r1 8
    mov r2 0
    draw r1 r2 5
    call wait
    mov r1 3
    ldf r1
    mov r1 16
    mov r2 0
    draw r1 r2 5
    call wait
    mov r1 4
    ldf r1
    mov r1 24
    mov r2 0
    draw r1 r2 5
    call wait
    mov r1 5
    ldf r1
    mov r1 32
    mov r2 0
    draw r1 r2 5
    call wait
    mov r1 6
    ldf r1
    mov r1 40
    mov r2 0
    draw r1 r2 5
    call wait
    mov r1 7
    ldf r1
    mov r1 48
    mov r2 0
    draw r1 r2 5

wait:
    mov r1 60
    delay r1
waitfor:
    ldd r9
    sne r9 0
    ret
    jmp waitfor
