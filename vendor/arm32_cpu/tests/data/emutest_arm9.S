@ vim:ft=armv4
@ expected result: 0x200 = 55, 0x204 = 66, 0x208 = 77
@ test for blx instruction

mov r2, #0x200
blx func
mov r0, #77
str r0, [r2, #8]
b end

func:
.thumb
mov r0, #55
mov r1, #66
str r0, [r2]
str r1, [r2, #4]
bx lr

.balign 4
end:
.arm
