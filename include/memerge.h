#ifndef __MEMERGE_H__
#define __MEMERGE_H__

#include <stddef.h>
#include <stdint.h>

typedef struct task_s task_t;

typedef enum {
    Smtp,
    Http,
} ParserType;

typedef enum {
  C2s,
  S2c,
  BiDir,
  Unknown
} PacketDir;

extern task_t *task_new();
extern void    task_free(task_t *task);
extern task_t *task_new_with_parser(ParserType parser_type);
extern task_t *task_init_parser(task_t *task, ParserType parser_type);
extern void    task_run(task_t *task, const u_int8_t *pkt, size_t pkt_len, PacketDir pkt_dir, uint64_t ts);

#endif
