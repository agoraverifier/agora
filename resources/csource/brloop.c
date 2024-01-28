// Use "-O0" arg to clang to avoid optimizations

#include <stdio.h>

void branch()
{
    int i = 1;
    if (i == 1)
    {
        printf("i is 1");
    }
    else
    {
        printf("i is not 1");
    }
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

int main(int argc, char const *argv[])
{
    branch();
    forloop();
    whileloop();
}
