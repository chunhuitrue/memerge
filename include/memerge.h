#ifndef __MEMERGE_H__
#define __MEMERGE_H__

#include <stdint.h>

typedef struct task_s task_t;

extern task_t *task_new();
extern void task_free(task_t *task);



extern uint32_t addition(uint32_t, uint32_t);

#endif
