/*
 * gcc --std=c11 -o c-example ffi.c -L../target/debug/ -lmemerge
 * LD_LIBRARY_PATH=target/debug/ ./c-example
 */
    
#include <stdio.h>
#include <stdint.h>

#include "../include/memerge.h"

int main(void) {
    task_t *task = NULL;


    task = task_new();
    if (!task) {
        printf("task new err\n");
    }
    printf("task new ok\n");
    init_parser(task, Smtp);
    printf("after init_parser\n");    
    task_free(task);
    task = NULL;


    task = task_new_with_parser(Http);
    if (task == NULL) {
        printf("task new_with_parser http. return NULL....ok\n");
    }

    task = task_new_with_parser(Smtp);
    if (task == NULL) {
        printf("task new_with_parser smtp. return NULL... error\n");
    }
    printf("task new_with_parser smtp. return... ok\n");
    task_free(task);
    task = NULL;
    
    
    return 0;
}
