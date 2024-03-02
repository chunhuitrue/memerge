#ifndef __MEMERGE_H__
#define __MEMERGE_H__

#include <stddef.h>
#include <stdint.h>

typedef struct task_s task_t;
typedef struct meta_s meta_t;

typedef enum {
    Smtp,
    Http,
    Undef        
} ParserType;

typedef enum {
  C2s,
  S2c,
  BiDir,
  Unknown
} PacketDir;

typedef enum {
    User,
    Pass,
    MailFrom,
    RcptTo,
    Subject,
    None,    
} MetaSmtpType;

extern task_t       *task_new();
extern void          task_free(task_t *task);
extern task_t       *task_new_with_parser(ParserType parser_type);
extern task_t       *task_init_parser(task_t *task, ParserType parser_type);
extern void          task_run(task_t *task, const u_int8_t *pkt, size_t pkt_len, PacketDir pkt_dir, uint64_t ts);
extern meta_t       *task_get_meta(task_t *task);
extern void          meta_free(meta_t *meta);
extern ParserType    meta_protocol(meta_t *meta);
extern MetaSmtpType  smtp_meta_type(meta_t *meta);
extern char         *smtp_meta_user(meta_t *meta);
extern void          smtp_meta_user_free(char *user);

#endif
