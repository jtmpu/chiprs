main:
    ldb r1 0
    add r1 2
    call func1
    call func2
    call func1
    exit

func1:
    add r1 4
    call func2
    ret

func2:
    add r1 2
    ret
