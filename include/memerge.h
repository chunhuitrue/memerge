#ifndef __MEMERGE_H__
#define __MEMERGE_H__

#include <stdint.h>

typedef struct task_s task_t;
typedef enum {
    Smtp,
    Http,
} ParserType;

extern task_t *task_new();
extern void task_free(task_t *task);
extern task_t *task_new_with_parser(ParserType parser_type);
extern void init_parser(task_t *task, ParserType parser_type);
#endif
