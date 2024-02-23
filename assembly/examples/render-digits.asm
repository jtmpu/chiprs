main:
    ldb r1 1
    ldf r1
    ldb r1 0
    ldb r2 0
    draw r1 r2 5
    ldb r1 0 
    ldf r1
    ldb r1 8
    ldb r2 0
    draw r1 r2 5
    ldb r1 3
    ldf r1
    ldb r1 16
    ldb r2 0
    draw r1 r2 5
    ldb r1 5 
    ldf r1
    ldb r1 24
    ldb r2 0
    draw r1 r2 5
    ldb r1 7
    ldf r1
    ldb r1 6
    ldb r2 6
    draw r1 r2 5
    ldb r1 9
    ldf r1
    ldb r1 12
    ldb r2 8
    draw r1 r2 5
loop:
    ldb r1 0
    jmp loop
    exit
