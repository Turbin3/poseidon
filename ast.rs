[
    Call(
        CallExpr {
            span: Span {
                lo: BytePos(
                    577,
                ),
                hi: BytePos(
                    605,
                ),
                ctxt: #0,
            },
            callee: Expr(
                Member(
                    MemberExpr {
                        span: Span {
                            lo: BytePos(
                                577,
                            ),
                            hi: BytePos(
                                589,
                            ),
                            ctxt: #0,
                        },
                        obj: Ident(
                            Ident {
                                span: Span {
                                    lo: BytePos(
                                        577,
                                    ),
                                    hi: BytePos(
                                        582,
                                    ),
                                    ctxt: #0,
                                },
                                sym: "state",
                                optional: false,
                            },
                        ),
                        prop: Ident(
                            Ident {
                                span: Span {
                                    lo: BytePos(
                                        583,
                                    ),
                                    hi: BytePos(
                                        589,
                                    ),
                                    ctxt: #0,
                                },
                                sym: "derive",
                                optional: false,
                            },
                        ),
                    },
                ),
            ),
            args: [
                ExprOrSpread {
                    spread: None,
                    expr: Array(
                        ArrayLit {
                            span: Span {
                                lo: BytePos(
                                    590,
                                ),
                                hi: BytePos(
                                    604,
                                ),
                                ctxt: #0,
                            },
                            elems: [
                                Some(
                                    ExprOrSpread {
                                        spread: None,
                                        expr: Lit(
                                            Str(
                                                Str {
                                                    span: Span {
                                                        lo: BytePos(
                                                            591,
                                                        ),
                                                        hi: BytePos(
                                                            597,
                                                        ),
                                                        ctxt: #0,
                                                    },
                                                    value: "vote",
                                                    raw: Some(
                                                        "\"vote\"",
                                                    ),
                                                },
                                            ),
                                        ),
                                    },
                                ),
                                Some(
                                    ExprOrSpread {
                                        spread: None,
                                        expr: Ident(
                                            Ident {
                                                span: Span {
                                                    lo: BytePos(
                                                        599,
                                                    ),
                                                    hi: BytePos(
                                                        603,
                                                    ),
                                                    ctxt: #0,
                                                },
                                                sym: "hash",
                                                optional: false,
                                            },
                                        ),
                                    },
                                ),
                            ],
                        },
                    ),
                },
            ],
            type_args: None,
        },
    ),
    Assign(
        AssignExpr {
            span: Span {
                lo: BytePos(
                    614,
                ),
                hi: BytePos(
                    644,
                ),
                ctxt: #0,
            },
            op: "=",
            left: Pat(
                Expr(
                    Member(
                        MemberExpr {
                            span: Span {
                                lo: BytePos(
                                    614,
                                ),
                                hi: BytePos(
                                    624,
                                ),
                                ctxt: #0,
                            },
                            obj: Ident(
                                Ident {
                                    span: Span {
                                        lo: BytePos(
                                            614,
                                        ),
                                        hi: BytePos(
                                            619,
                                        ),
                                        ctxt: #0,
                                    },
                                    sym: "state",
                                    optional: false,
                                },
                            ),
                            prop: Ident(
                                Ident {
                                    span: Span {
                                        lo: BytePos(
                                            620,
                                        ),
                                        hi: BytePos(
                                            624,
                                        ),
                                        ctxt: #0,
                                    },
                                    sym: "vote",
                                    optional: false,
                                },
                            ),
                        },
                    ),
                ),
            ),
            right: Call(
                CallExpr {
                    span: Span {
                        lo: BytePos(
                            627,
                        ),
                        hi: BytePos(
                            644,
                        ),
                        ctxt: #0,
                    },
                    callee: Expr(
                        Member(
                            MemberExpr {
                                span: Span {
                                    lo: BytePos(
                                        627,
                                    ),
                                    hi: BytePos(
                                        641,
                                    ),
                                    ctxt: #0,
                                },
                                obj: Member(
                                    MemberExpr {
                                        span: Span {
                                            lo: BytePos(
                                                627,
                                            ),
                                            hi: BytePos(
                                                637,
                                            ),
                                            ctxt: #0,
                                        },
                                        obj: Ident(
                                            Ident {
                                                span: Span {
                                                    lo: BytePos(
                                                        627,
                                                    ),
                                                    hi: BytePos(
                                                        632,
                                                    ),
                                                    ctxt: #0,
                                                },
                                                sym: "state",
                                                optional: false,
                                            },
                                        ),
                                        prop: Ident(
                                            Ident {
                                                span: Span {
                                                    lo: BytePos(
                                                        633,
                                                    ),
                                                    hi: BytePos(
                                                        637,
                                                    ),
                                                    ctxt: #0,
                                                },
                                                sym: "vote",
                                                optional: false,
                                            },
                                        ),
                                    },
                                ),
                                prop: Ident(
                                    Ident {
                                        span: Span {
                                            lo: BytePos(
                                                638,
                                            ),
                                            hi: BytePos(
                                                641,
                                            ),
                                            ctxt: #0,
                                        },
                                        sym: "sub",
                                        optional: false,
                                    },
                                ),
                            },
                        ),
                    ),
                    args: [
                        ExprOrSpread {
                            spread: None,
                            expr: Lit(
                                Num(
                                    Number {
                                        span: Span {
                                            lo: BytePos(
                                                642,
                                            ),
                                            hi: BytePos(
                                                643,
                                            ),
                                            ctxt: #0,
                                        },
                                        value: 1.0,
                                        raw: Some(
                                            "1",
                                        ),
                                    },
                                ),
                            ),
                        },
                    ],
                    type_args: None,
                },
            ),
        },
    ),
]