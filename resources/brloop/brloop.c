// Use "-O0" arg to clang to avoid optimizations

#include <stdio.h>

int branch(int i)
{
    int b = 0;
    switch (i)
    {
    case 1:
        b = 0x55;
        break;
    case 2:
        b = 0x66;
        break;
    case 5:
        b = 0xffff;
        break;
    case 0:
        b = 0xeeee;
        break;
    default:
        b = 0x77;
        break;
    }
    return b;
}

void forloop()
{
    int i;
    int a[10];
    for (i = 0; i < 10; i++)
    {
        a[i] = i;
    }
}

void whileloop()
{
    int i = 0;
    while (i < 10)
    {
        printf("i is %d", i);
        i++;
    }
}

void if_test()
{
    int i = 0x55;
    if (i < 0x10)
    {
        i++;
    }
}

int main(int argc, char const *argv[])
{
    branch(3);
    forloop();
    whileloop();
    if_test();
}
