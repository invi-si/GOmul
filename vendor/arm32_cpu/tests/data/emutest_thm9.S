@ vim:ft=armv4
@ assume SP starts at 200
@ expected result: 0x200 = 11, 0x204 = 22
@ blx test

.thumb
mov r0, #11
mov r1, #22
mov r2, #12
blx r2
b end

.balign 4
test:
.arm
mov r2, #0x200
str r0, [r2]
str r1, [r2, #4]
bx lr

end:
