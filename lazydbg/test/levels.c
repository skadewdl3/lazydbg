// callchain.c

#include <stdio.h>
#include <stdlib.h>

void level10(int x);
void level1(int x);
void level2(int x);
void level3(int x);
void level4(int x);
void level5(int x);
void level6(int x);
void level7(int x);
void level8(int x);
void level9(int x);

void level1(int x) {
    printf("level1: %d\n", x);
    level2(x + 1);
}

void level2(int x) {
    printf("level2: %d\n", x);
    level3(x + 1);
}

void level3(int x) {
    printf("level3: %d\n", x);
    level4(x + 1);
}

void level4(int x) {
    printf("level4: %d\n", x);
    level5(x + 1);
}

void level5(int x) {
    printf("level5: %d\n", x);
    level6(x + 1);
}

void level6(int x) {
    printf("level6: %d\n", x);
    level7(x + 1);
}

void level7(int x) {
    printf("level7: %d\n", x);
    level8(x + 1);
}

void level8(int x) {
    printf("level8: %d\n", x);
    level9(x + 1);
}

void level9(int x) {
    printf("level9: %d\n", x);
    level10(x + 1);
}

void level10(int x) {
    printf("level10: %d\n", x);

    // Useful breakpoint target
    volatile int breakpoint_here = x * 2;

    printf("final value: %d\n", breakpoint_here);
}

int main(void) {
    printf("starting call chain\n");

    int value = 42;
    level1(value);

    printf("done\n");
    return 0;
}
