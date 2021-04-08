local actions = require('telescope.actions')
local action_state = require('telescope.actions.state')
local finders = require('telescope.finders')
local pickers = require('telescope.pickers')
local sorters = require('telescope.sorters')
local previewers = require('telescope.previewers')

function mbox_picker(mboxes)
  pickers.new {
    results_title = 'Mailboxes',
    finder = finders.new_table {
        results = mboxes,
        entry_maker = function(entry)
            return {
                value = entry,
                display = entry,
                ordinal = entry,
                preview_command = function(entry, bufnr)
                    vim.api.nvim_buf_call(bufnr, function()
                        local page = 0 -- page 0 for preview
                        local success, output = pcall(vim.fn['himalaya#msg#list_with'], entry.value, page, true)
                        if not (success) then
                            vim.cmd('redraw')
                            vim.bo.modifiable = true
                            local errors = vim.fn.split(output, '\n')
                            errors[1] = "Errors: "..errors[1]
                            vim.api.nvim_buf_set_lines(bufnr, 0, -1, true, errors)
                        end
                    end)
                end
            }
      end,
    },
    sorter = sorters.fuzzy_with_index_bias(),
    attach_mappings = function(prompt_bufnr)
      actions.select_default:replace(function()
        local selection = action_state.get_selected_entry()
        actions.close(prompt_bufnr)
        vim.fn['himalaya#mbox#post_input'](selection.display)
      end)

      return true
    end,
    previewer = previewers.display_content.new({})
  }:find()
end
