#include <stdio.h>
#include <stdint.h>
#include <pcap.h>
#include <netinet/ip.h>
#include <netinet/tcp.h>

#include "../include/memerge.h"

int simple(void) {
    task_t  *task  = NULL;
    uint8_t  pkt[] = {1,2,3,4,5,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1};
    meta_t  *meta;
    
    task = task_new();
    if (!task) {
        printf("task new err\n");
    }
    printf("task new ok\n");
    task_init_parser(task, Smtp);
    printf("after task_init_parser\n");    
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

    printf("task run. pktlen: %lu\n", sizeof(pkt));
    task_run(task, pkt, sizeof(pkt), C2s, 999);
    printf("task run. 222\n");

    meta = task_get_meta(task);
    if (meta == NULL) {
        printf("meta is null.\n");
        return 0;
    }

    if (meta_protocol(meta) == Smtp) {
        
    }
    
    meta_free(meta);
    task_free(task);
    task = NULL;
    return 0;
}

#define SMTP_PCAP "../tests/smtp.pcap"
task_t *task = NULL;

int need_pkt(uint8_t *pkt) {
    struct ip     *ip_header;
    struct tcphdr *tcp_header;

    ip_header = (struct ip *)(pkt + 14);
    if (ip_header->ip_p == IPPROTO_TCP) {
        tcp_header = (struct tcphdr *)(pkt + 14 + ip_header->ip_hl * 4);
        if (ntohs(tcp_header->th_dport) == 25) {
            return 1;
        }
    }
    return 0; 
}

void packet_handler(u_char *user_data, const struct pcap_pkthdr *pkthdr, const u_char *packet) {
    meta_t       *meta;
    char         *meta_user;
    MetaSmtpType  smtp_type;
    
    if (task == NULL) {
        printf("packet_handler. task is null, return.\n");
        return;
    }
    if (need_pkt((uint8_t *)packet) == 0) {
        return;
    }
    
    task_run(task, packet, pkthdr->len, C2s, 999);    
    meta = task_get_meta(task);
    if (meta == NULL) {
        return;
    }
    if (meta_protocol(meta) != Smtp) {
        return;
    }

    smtp_type = smtp_meta_type(meta);
    switch (smtp_type) {
    case User:
        meta_user = smtp_meta_user(meta);
        if (meta_user) {
            printf("get smtp_meta_user: %s\n", meta_user);
            smtp_meta_user_free(meta_user);
        }
        break;
    case Pass:
        break;
    case MailFrom:
        break;
    case RcptTo:
        break;
    case Subject:
        break;
    default:
        break;
    }
    
    meta_free(meta);
}

int main(void) {
    task = task_new_with_parser(Smtp);
    if (task == NULL) {
        fprintf(stderr, "Error task_init_parser.\n");
        return 1;
    }
    
    char    errbuf[PCAP_ERRBUF_SIZE];
    pcap_t *pcap = pcap_open_offline(SMTP_PCAP, errbuf);
    if (pcap == NULL) {
        fprintf(stderr, "Error opening pcap file: %s\n", errbuf);
        return 1;
    }
    if (pcap_loop(pcap, 0, packet_handler, NULL) < 0) {
        fprintf(stderr, "Error in pcap_loop\n");
        return 1;
    }

    task_free(task);
    pcap_close(pcap);
    return 0;;
}
