@ vim:ft=armv4
@ assume SP starts at 200
@ expected result: 0x1fc = 0x55
@ regression test for ADD PC, Rm using PC+4 in Thumb

.thumb
mov r1, #2
add pc, r1
nop
b fail
mov r0, #0x55
b end
fail:
mov r0, #0
end:
push {r0}
