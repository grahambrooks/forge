package com.acme;

import org.springframework.web.bind.annotation.*;

@RestController
@RequestMapping("/api")
public class PaymentController {

    @GetMapping("/payments")
    public String listPayments() {
        return "[]";
    }

    @PostMapping("/payments")
    public String createPayment() {
        return "{}";
    }

    @GetMapping("/payments/{id}")
    public String getPayment(@PathVariable String id) {
        return "{}";
    }
}
