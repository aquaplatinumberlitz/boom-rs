set pagination off
set confirm off
break *0x446430
commands
silent
printf "TRAP_ARG ms=%d ticks=%d slot=%d\n", *(int*)($ebp+0xc), $edi, *(int*)($ebp+8)
continue
end
break *0x574802
commands
silent
printf "BUBBLE_MAX max=%d obj=%p\n", $eax, $esi
continue
end
break *0x7172E7
commands
silent
printf "DEATH_PKT_120_13\n"
continue
end
continue
