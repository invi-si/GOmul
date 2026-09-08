.thumb
mov r0, #11
mov r1, #22
blx test
b end

.balign 4
test:
.arm
str r0, [sp]
str r1, [sp, #4]
bx lr

end:
