// Use "-O0" arg to clang to avoid optimizations

#include <stdio.h>

void foo()
{
    printf("foo\n");
    return;
}

int main(int argc, char const *argv[])
{
    void (*fun_ptr)() = &foo;
    (*fun_ptr)();

    return 0;
}
