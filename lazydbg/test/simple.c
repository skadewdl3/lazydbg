#include <stdio.h>

int main() {
    int *ptr = NULL; // Create a pointer pointing to nothing
    *ptr = 42;       // Dereference the NULL pointer (CRASH)
    return 0;
}
