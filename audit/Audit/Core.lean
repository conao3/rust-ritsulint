import Assay
import Ritsulint.Properties

set_option assay.maxSize 9999 in
#assay Ritsulint.kansujiStep Ritsulint.parseKansuji Ritsulint.kansujiOr0
  using Ritsulint.parseKansuji_toKansuji Ritsulint.kansujiOr0_toKansuji

#assay Ritsulint.unquoteAux Ritsulint.unquote
  using Ritsulint.unquote_plain Ritsulint.unquote_quotation

#assay Ritsulint.stripParensAux Ritsulint.stripParens
  using Ritsulint.stripParens_plain Ritsulint.stripParens_remark
