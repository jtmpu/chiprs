main:
    call wait
    ldb r1 1
    ldf r1
    ldb r1 0
    ldb r2 0
    draw r1 r2 5
    call wait
    ldb r1 2
    ldf r1
    ldb r1 8
    ldb r2 0
    draw r1 r2 5
    call wait
    ldb r1 3
    ldf r1
    ldb r1 16
    ldb r2 0
    draw r1 r2 5
    call wait
    ldb r1 4
    ldf r1
    ldb r1 24
    ldb r2 0
    draw r1 r2 5
    call wait
    ldb r1 5
    ldf r1
    ldb r1 32
    ldb r2 0
    draw r1 r2 5
    call wait
    ldb r1 6
    ldf r1
    ldb r1 40
    ldb r2 0
    draw r1 r2 5
    call wait
    ldb r1 7
    ldf r1
    ldb r1 48
    ldb r2 0
    draw r1 r2 5

wait:
    ldb r1 60
    delay r1
waitfor:
    ldd r9
    sne r9 0
    ret
    jmp waitfor
